use crate::types::{Price, PriceList, PriceSource, Symbol};
use alloc::{string::String, vec, vec::Vec};
use core::str::from_utf8;
use frame_support::{traits::ConstU32, BoundedVec};
use lite_json::json::JsonValue;
use sp_runtime::offchain::{http, Duration, Timestamp};

pub struct CryptoCompare;

impl PriceSource for CryptoCompare {
	fn name(&self) -> &'static str {
		"CryptoCompare"
	}
	fn weight(&self) -> u32 {
		5
	}
	fn fetch(&self, symbols: &[String]) -> Result<PriceList, http::Error> {
		let base_str_query = symbols.join(",");
		// https://min-api.cryptocompare.com/data/pricemulti?fsyms=BTC,ETH&tsyms=USD
		let cryptocompare_url = alloc::format!(
			"https://min-api.cryptocompare.com/data/pricemulti?fsyms={}&tsyms=USD",
			base_str_query
		);

		let deadline: sp_core::offchain::Timestamp =
			sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
		let request = http::Request::get(&cryptocompare_url);

		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;

		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown);
		}

		let body = response.body().collect::<Vec<u8>>();

		let body_str = from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		parse_cryptocompare_prices(body_str).ok_or(http::Error::Unknown)
	}
}

fn parse_cryptocompare_prices(
	price_str: &str,
) -> Option<Vec<(BoundedVec<u8, ConstU32<32>>, u128)>> {
	let val = lite_json::parse_json(price_str).ok()?;
	let obj = match val {
		JsonValue::Object(o) => o,
		_ => return None,
	};

	let mut out = Vec::new();
	for (key_chars, value) in obj.into_iter() {
		let symbol_str: String = key_chars.into_iter().collect();
		let symbol_bytes = symbol_str.into_bytes();

		let bounded_symbol: BoundedVec<u8, ConstU32<32>> = symbol_bytes.try_into().ok()?;

		let usd_number = if let JsonValue::Object(inner) = value {
			let (_, v) = inner.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
			match v {
				JsonValue::Number(n) => n,
				_ => return None,
			}
		} else {
			return None;
		};

		let exp = usd_number.fraction_length.saturating_sub(2);
		let price_cents =
			usd_number.integer as u128 * 100 + (usd_number.fraction / 10_u64.pow(exp)) as u128;
		log::info!("price_cents {:?}", price_cents);

		out.push((bounded_symbol, price_cents));
	}

	Some(out)
}
