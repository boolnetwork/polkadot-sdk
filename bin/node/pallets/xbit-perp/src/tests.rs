use crate::{mock::{new_test_ext, RuntimeOrigin as Origin, XbitPerp}, Error, Market};
use frame_support::{assert_err, assert_ok, pallet_prelude::*};
use frame_system::Pallet as System;
use sp_std::vec;
use crate::mock::Test;

#[test]
fn test_create_market() {
	new_test_ext().execute_with(|| {
		let mut market = Market {
			id: 1,
			contract_address: vec![1, 2, 3],
			save_time: 1000,
			pair: b"BTC/USDT".to_vec(),
			token_a: b"BTC".to_vec(),
			token_a_address: vec![1, 2, 3],
			token_a_decimal: 8,
			token_b: b"USDT".to_vec(),
			token_b_address: vec![4, 5, 6],
			token_b_decimal: 6,
			network: b"ethereum".to_vec(),
			signer_key: vec![1, 2, 3],
			height: 1000,
			substrate: true,
			substrate_pair: b"BTC/USDT".to_vec(),
			cumulative_funding_rate: 0,
			last_cacl_funding_rate_time: 1000,
			oracle_price: 50000000000000000000, // 50 USDT
			max_deviation_bps: 100,
			liquid_spread_bps: 50,
			fallback_if_dlob_price_invalid: false,
			maintenance_margin_ratio: 50000000000000000, // 5%
		};

		assert_ok!(XbitPerp::create_market(Origin::root(), market.clone()));
		assert_eq!(XbitPerp::market_info(1), Some(market.clone()));
		assert_eq!(XbitPerp::market_count(), 1);

		market.oracle_price = 100;
		assert_ne!(XbitPerp::market_info(1), Some(market.clone()));
		assert_ok!(XbitPerp::update_market(Origin::root(), 1, market.clone()));
		assert_err!(XbitPerp::update_market(Origin::root(), 2, market.clone()),  Error::<Test>::MarketNotFound);
		assert_eq!(XbitPerp::market_info(2), None);
		assert_err!(XbitPerp::create_market(Origin::root(), market.clone()), Error::<Test>::MarketAlreadyExists);
		market.id = 2;
		assert_ok!(XbitPerp::create_market(Origin::root(), market));
		assert_eq!(XbitPerp::market_count(), 2);

		let events = System::<Test>::events();
		for event in &events {
			println!("Event: {:#?}", event);
		}
	})
}
