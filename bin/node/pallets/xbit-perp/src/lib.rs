#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
pub mod weights;


pub use weights::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
    use super::*;
    use sp_std::{vec, vec::Vec};
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::Hash;
    use frame_system::pallet_prelude::*;
    use scale_info::prelude::format;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type Call: From<Call<Self>>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        MarketCreated { market_id: u64, market: Market },
        MarketUpdated { market_id: u64, market: Market },
    }

    #[pallet::error]
    pub enum Error<T> {
        MarketNotFound,
        InvalidMarketId,
        MarketAlreadyExists,
    }


    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Default, scale_info::TypeInfo)]
    pub struct Market {
        /// market id
        pub id: u64,
        /// market contract address
        pub contract_address: Vec<u8>,
        /// save time
        pub save_time: u64,
        /// pair name (e.g. BTC/USDT)
        pub pair: Vec<u8>,
        /// token a symbol (e.g. BTC)
        pub token_a: Vec<u8>,
        /// token a address
        pub token_a_address: Vec<u8>,
        /// token a decimal
        pub token_a_decimal: i32,
        /// token b symbol (e.g. USDT)
        pub token_b: Vec<u8>,
        /// token b address
        pub token_b_address: Vec<u8>,
        /// token b decimal
        pub token_b_decimal: i32,
        /// network (e.g. ethereum, bsc.)
        pub network: Vec<u8>,
        /// signer key
        pub signer_key: Vec<u8>,
        /// height (deployed block height)
        pub height: u64,
        /// substrate (if the market is deployed on substrate. 1: substrate, 0: not substrate)
        pub substrate: bool,
        /// substrate pair (the pair name of substrate)
        pub substrate_pair: Vec<u8>,
        /// cumulative funding rate
        pub cumulative_funding_rate: i128,
        /// last cacl funding rate time
        pub last_cacl_funding_rate_time: u64,
        /// current oracle price, units: quote asset (e.g. USDT), decimal: 1e18
        pub oracle_price: u128,
        /// max deviation bps
        pub max_deviation_bps: u64,
        /// liquid spread bps
        pub liquid_spread_bps: u64,
        /// fallback if dlob price invalid
        pub fallback_if_dlob_price_invalid: bool,
        /// maintenance margin ratio
        pub maintenance_margin_ratio: u128,
    }


    #[pallet::storage]
    #[pallet::getter(fn market_info)]
    pub(super) type MarketStorage<T: Config> = StorageMap<_, Twox64Concat, u64, Market>;

    #[pallet::storage]
    #[pallet::getter(fn market_count)]
    pub(super) type MarketCount<T: Config> = StorageValue<_, u64, ValueQuery>;



    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create_market(
            origin: OriginFor<T>,
            market: Market,
        ) -> DispatchResult {
            ensure_root(origin)?;
            let market_id = market.id;
            ensure!(!MarketStorage::<T>::contains_key(market_id), Error::<T>::MarketAlreadyExists);

            MarketStorage::<T>::insert(market_id, market.clone());
            let current_count = MarketCount::<T>::get();
            let new_count = current_count + 1;
            MarketCount::<T>::put(new_count);

            Self::deposit_event(Event::MarketCreated { market_id, market });

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn update_market(
            origin: OriginFor<T>,
            market_id: u64,
            market: Market,
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;
            ensure!(MarketStorage::<T>::contains_key(market_id), Error::<T>::MarketNotFound);

            MarketStorage::<T>::insert(market_id, market.clone());

            Self::deposit_event(Event::MarketUpdated { market_id, market });

            Ok(())
        }
    }
}
