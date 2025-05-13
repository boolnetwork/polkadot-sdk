use crate::pallet as xbit_perp;
use sp_std::vec;
use frame_support::{parameter_types, sp_io, traits::{ConstU32, ConstU64}};
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		XbitPerp: xbit_perp::{Pallet, Call, Storage, Event<T>},
	}
);

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;



parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl xbit_perp::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Call = RuntimeCall;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
//
// #[test]
// fn test_create_market() {
// 	new_test_ext().execute_with(|| {
// 		let market = Market {
// 			id: 1,
// 			contract_address: vec![1, 2, 3],
// 			save_time: 1000,
// 			pair: b"BTC/USDT".to_vec(),
// 			token_a: b"BTC".to_vec(),
// 			token_a_address: vec![1, 2, 3],
// 			token_a_decimal: 8,
// 			token_b: b"USDT".to_vec(),
// 			token_b_address: vec![4, 5, 6],
// 			token_b_decimal: 6,
// 			network: b"ethereum".to_vec(),
// 			signer_key: vec![1, 2, 3],
// 			height: 1000,
// 			substrate: true,
// 			substrate_pair: b"BTC/USDT".to_vec(),
// 			cumulative_funding_rate: 0,
// 			last_cacl_funding_rate_time: 1000,
// 			oracle_price: 50000000000000000000, // 50 USDT
// 			max_deviation_bps: 100,
// 			liquid_spread_bps: 50,
// 			fallback_if_dlob_price_invalid: false,
// 			maintenance_margin_ratio: 50000000000000000, // 5%
// 		};
//
// 		assert_ok!(XbitPerpMarket::create_market(Origin::root(), market.clone()));
// 		assert_eq!(XbitPerpMarket::market_info(1), Some(market));
// 	});
// }
//
// #[test]
// fn test_update_market() {
// 	new_test_ext().execute_with(|| {
// 		let market = Market {
// 			id: 1,
// 			contract_address: vec![1, 2, 3],
// 			save_time: 1000,
// 			pair: b"BTC/USDT".to_vec(),
// 			token_a: b"BTC".to_vec(),
// 			token_a_address: vec![1, 2, 3],
// 			token_a_decimal: 8,
// 			token_b: b"USDT".to_vec(),
// 			token_b_address: vec![4, 5, 6],
// 			token_b_decimal: 6,
// 			network: b"ethereum".to_vec(),
// 			signer_key: vec![1, 2, 3],
// 			height: 1000,
// 			substrate: true,
// 			substrate_pair: b"BTC/USDT".to_vec(),
// 			cumulative_funding_rate: 0,
// 			last_cacl_funding_rate_time: 1000,
// 			oracle_price: 50000000000000000000,
// 			max_deviation_bps: 100,
// 			liquid_spread_bps: 50,
// 			fallback_if_dlob_price_invalid: false,
// 			maintenance_margin_ratio: 50000000000000000,
// 		};
//
// 		assert_ok!(XbitPerpMarket::create_market(Origin::root(), market.clone()));
//
// 		let updated_market = Market {
// 			oracle_price: 55000000000000000000, // 55 USDT
// 			..market.clone()
// 		};
//
// 		assert_ok!(XbitPerpMarket::update_market(Origin::root(), 1, updated_market.clone()));
// 		assert_eq!(XbitPerpMarket::market_info(1), Some(updated_market));
// 	});
// }
//
// #[test]
// fn test_get_market() {
// 	new_test_ext().execute_with(|| {
// 		let market = Market {
// 			id: 1,
// 			contract_address: vec![1, 2, 3],
// 			save_time: 1000,
// 			pair: b"BTC/USDT".to_vec(),
// 			token_a: b"BTC".to_vec(),
// 			token_a_address: vec![1, 2, 3],
// 			token_a_decimal: 8,
// 			token_b: b"USDT".to_vec(),
// 			token_b_address: vec![4, 5, 6],
// 			token_b_decimal: 6,
// 			network: b"ethereum".to_vec(),
// 			signer_key: vec![1, 2, 3],
// 			height: 1000,
// 			substrate: true,
// 			substrate_pair: b"BTC/USDT".to_vec(),
// 			cumulative_funding_rate: 0,
// 			last_cacl_funding_rate_time: 1000,
// 			oracle_price: 50000000000000000000,
// 			max_deviation_bps: 100,
// 			liquid_spread_bps: 50,
// 			fallback_if_dlob_price_invalid: false,
// 			maintenance_margin_ratio: 50000000000000000,
// 		};
//
// 		assert_ok!(XbitPerpMarket::create_market(Origin::root(), market.clone()));
// 		assert_ok!(XbitPerpMarket::get_market(1));
// 	});
// }
