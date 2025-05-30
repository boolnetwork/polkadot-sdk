#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;
pub mod datasources;
pub mod types;

use types::{Symbol, Price};

use codec::{Decode, Encode};
use frame_support::traits::Get;
use frame_system::{
	self as system,
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SigningTypes, SubmitTransaction,
	},
};
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	offchain::{
		http,
		storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
		storage_lock::{StorageLock, Time}
	},
	traits::Zero,
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeDebug,
};

use sp_std::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use crate::alloc::string::ToString;
use core::str::from_utf8;

use sp_runtime::{
	 MultiSigner,
	 AccountId32,
};
use sp_io::crypto::sr25519_public_keys;
use sp_runtime::traits::IdentifyAccount;


pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");

pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct AuthorityId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuthorityId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for AuthorityId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: BlockNumberFor<T>) {
            let signer = Signer::<T, T::AuthorityId>::any_account();
                        
            if !signer.can_sign() {
                return;
            }

			// 获取公钥
			let pubkeys = sr25519_public_keys(KEY_TYPE);
			if pubkeys.is_empty() {
				log::warn!("No keys found for KEY_TYPE");
				return;
			}
			
			let account_id = MultiSigner::Sr25519(pubkeys[0]).into_account();
			
			// 检查白名单
			if !Whitelist::<T>::contains_key(&account_id) {
				log::info!("Account not whitelisted: {:?}", account_id);
				return;
			}
           
			let seed = sp_io::offchain::random_seed();
            let extra = u64::from_le_bytes(seed.as_ref()[0..8].try_into().unwrap()) % 8000;
            let delay = Duration::from_millis(6_000 + extra);
			{
                let mut lock = StorageLock::<Time>::with_deadline(b"btc_http_lock", delay);

                if let Ok(_guard) = lock.try_lock() {
                    {
                        sp_io::offchain::sleep_until(sp_io::offchain::timestamp().add(delay));
                        Self::fetch_and_store();
						// 2. 从 offchain 存储读出刚才存的 entries
						let storage_ref = StorageValueRef::persistent(b"price_entries");
						let maybe_bytes = storage_ref.get::<Vec<(Vec<u8>, Vec<u8>, u128, u32)>>();
						let entries = match maybe_bytes {
							Ok(Some(v)) => v,
							_ => {
								log::warn!("No price entries in offchain storage");
								return;
							}
						};

						// 3. 按 symbol 分组，累加 sum(price * weight) 和 sum(weight)
						let mut accum: BTreeMap<Vec<u8>, (u128, u32)> = BTreeMap::new();
						for (_src, symbol_bytes, price, weight) in entries.into_iter() {
							let entry = accum.entry(symbol_bytes.clone()).or_insert((0u128, 0u32));
							entry.0 = entry.0.saturating_add(price.saturating_mul(weight as u128));
							entry.1 = entry.1.saturating_add(weight);
						}

						// 3. 计算加权平均，并构造 submit_price 的 inputs
						let mut inputs: Vec<(BoundedVec<u8, ConstU32<32>>, u128)> = Vec::new();
						for (sym_bytes, (sum_pw, sum_w)) in accum.into_iter() {
							if sum_w == 0 {
								continue;
							}
							let avg_price = sum_pw / (sum_w as u128);
							match BoundedVec::<u8, ConstU32<32>>::try_from(sym_bytes.clone()) {
								Ok(bounded_sym) => {
                                    log::info!("inputs,inputs {:?}", inputs);
									inputs.push((bounded_sym, avg_price));
								}
								Err(_) => {
									log::warn!("Symbol too long, skip: {:?}", sym_bytes);
								}
							}
						}

						// 4. 提交到链上
						if !inputs.is_empty() {
							let ts = sp_io::offchain::timestamp().unix_millis();
							if let Some((_acct, Ok(()))) = 
								Signer::<T, T::AuthorityId>::any_account()
									.send_signed_transaction(|_acct| {
										Call::submit_price { inputs: inputs.clone(), timestamp: ts }
									})
							{
								log::info!("submit_price 成功，inputs={:?}", inputs);
							} else {
								log::warn!("submit_price 失败");
							}
						}
                    }
                };
            }
		}
	}

	#[pallet::storage]
    #[pallet::getter(fn supported_symbols)]
    pub(super) type SupportedSymbols<T: Config> = StorageValue<
        _,
        BoundedVec<BoundedVec<u8, ConstU32<32>>, ConstU32<64>>,
        ValueQuery,
        DefaultSupportedSymbols<T>
    >;

    #[pallet::type_value]
    pub(super) fn DefaultSupportedSymbols<T: Config>() -> BoundedVec<BoundedVec<u8, ConstU32<32>>, ConstU32<64>> {
        vec![
            b"BTC/USDT".to_vec().try_into().unwrap(),
            b"ETH/USDT".to_vec().try_into().unwrap(),
            b"SOL/USDT".to_vec().try_into().unwrap(),
            b"DOGE/USDT".to_vec().try_into().unwrap(),
        ]
        .try_into()
        .unwrap()
    }

    
    /// Last reported price (weighted)
    #[pallet::storage]
    #[pallet::getter(fn prices)]
    pub type LastPrices<T: Config> = StorageMap<_, Blake2_128Concat, BoundedVec<u8, ConstU32<32>>, (u128, u64), OptionQuery>;

    /// Whitelisted node indices allowed to fetch and report
    #[pallet::storage]
    #[pallet::getter(fn whitelist)]
    pub type Whitelist<T: Config> = StorageMap<_, Blake2_128Concat, AccountId32, (), OptionQuery>;



    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        PriceUpdated(u128, u64),
        WhitelistSet(Vec<AccountId32>),
        Whitelisted(AccountId32),
        Unwhitelisted(AccountId32),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Offchain signer unavailable
        NoSigner,
        /// HTTP fetch error
        HttpFetchError,
        /// JSON parse error
        JsonParseError,
        /// Arithmetic overflow
        ArithmeticOverflow,
        NodeNotWhitelisted,
        TooManySymbols,
        SymbolTooLong,
        SymbolAlreadyExists,
    }


    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({10_000})]
        pub fn submit_price(
            origin: OriginFor<T>,
            inputs: Vec<(BoundedVec<u8, ConstU32<32>>, u128)>,
            timestamp: u64,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            for (symbol, price) in inputs.into_iter() {
                LastPrices::<T>::insert(symbol.clone(), (price, timestamp));
            }
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn add_whitelist(origin: OriginFor<T>, who: AccountId32) -> DispatchResult {
            ensure_root(origin)?;
            Whitelist::<T>::insert(&who, ());
            Self::deposit_event(Event::Whitelisted(who));
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn remove_whitelist(origin: OriginFor<T>, who: AccountId32) -> DispatchResult {
            ensure_root(origin)?;
            ensure!(Whitelist::<T>::contains_key(&who), Error::<T>::NodeNotWhitelisted);
            Whitelist::<T>::remove(&who);
            Self::deposit_event(Event::Unwhitelisted(who));
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn add_symbol(origin: OriginFor<T>, symbol: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?; 
            let bounded_symbol: BoundedVec<u8, ConstU32<32>> = symbol.try_into()
                .map_err(|_| Error::<T>::SymbolTooLong)?;

            SupportedSymbols::<T>::try_mutate(|symbols| {
                if symbols.contains(&bounded_symbol) {
                    Err(Error::<T>::SymbolAlreadyExists)
                } else {
                    symbols.try_push(bounded_symbol.clone()).map_err(|_| Error::<T>::TooManySymbols)?;
                    Ok(())
                }
            })?;

            log::info!("✅ Symbol added: {:?}", bounded_symbol);
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(10_000)]
        pub fn remove_symbol(origin: OriginFor<T>, symbol: Vec<u8>) -> DispatchResult {
            ensure_root(origin)?;
            let bounded_symbol: BoundedVec<u8, ConstU32<32>> = symbol.try_into()
                .map_err(|_| Error::<T>::SymbolTooLong)?;

            SupportedSymbols::<T>::mutate(|symbols| {
                if let Some(index) = symbols.iter().position(|s| *s == bounded_symbol) {
                    symbols.swap_remove(index);
                }
            });

            log::info!("🗑️ Symbol removed: {:?}", bounded_symbol);
            Ok(())
        }
       
    }

}

impl<T: Config> Pallet<T> {
	pub fn fetch_and_store() -> Result<(), http::Error> {
		let mut base_tokens: Vec<String> = Vec::new();
        for symbol in SupportedSymbols::<T>::get().iter() {
            let raw: Vec<u8> = symbol.to_vec();
            
            if let Some(pos) = raw.iter().position(|b| *b == b'/') {
                let base = from_utf8(&raw[0..pos]).map_err(|_| http::Error::Unknown)?.to_string();
                base_tokens.push(base);
            }
        }

        if base_tokens.is_empty() {
            return Err(http::Error::Unknown);
        }

		// 准备存储的条目列表：(source_name, symbol_str, price)
		let mut entries: Vec<(Vec<u8>, Symbol, u128, u32)> = Vec::new();
	
		// 1. 获取所有激活的数据源
		let sources = datasources::active_sources();
		for src in sources {
			let w = src.weight();
			if w == 0 {
				continue;
			}
			// 假设 base_tokens 已经准备好
			if let Ok(list) = src.fetch(&base_tokens) {
				for (symbol, price) in list {
					// 过滤旧价偏差 >5% 的逻辑也可以在这里做
					if let Some((last_price, _ts)) = LastPrices::<T>::get(&symbol) {
						if last_price > 0 {
							let diff = if price > last_price {
								price - last_price
							} else {
								last_price - price
							};
							if diff.saturating_mul(100) >= last_price.saturating_mul(5) {
								log::warn!(
									"Price Over 5%, Ignore data: {:?}, new={} last={}",
									symbol, price, last_price
								);
								continue;
							}
						}
					}
					entries.push((
						src.name().as_bytes().to_vec(), 
						symbol.clone(),               
						price,                        
						w,                            
					));
				}
			}
		}
	
		// 3. 将 entries 序列化后写入本地 Offchain 存储
		let storage_ref = StorageValueRef::persistent(b"price_entries");
		storage_ref.set(&entries);
	
		Ok(())
	}

}