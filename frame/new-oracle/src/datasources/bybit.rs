use super::parse_price_to_cents;
use crate::types::{Price, PriceList, PriceSource, Symbol};
use alloc::{
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};
use core::str::from_utf8;
use frame_support::{traits::ConstU32, BoundedVec};
use lite_json::json::JsonValue;
use microjson::{JSONValue, JSONValueType};
use sp_runtime::offchain::{http, Duration, Timestamp};

pub struct Bybit;

impl PriceSource for Bybit {
	fn name(&self) -> &'static str {
		"Bybit"
	}
	fn weight(&self) -> u32 {
		45
	}

	fn fetch(&self, symbols: &[String]) -> Result<PriceList, http::Error> {
		let bt_url = "https://api.bybit.com/v5/market/tickers?category=linear";
		// let bt_url = "http://localhost:3000/bybit";

		let deadline: sp_core::offchain::Timestamp =
			sp_io::offchain::timestamp().add(Duration::from_millis(10_000));
		let request = http::Request::get(&bt_url);

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

		parse_bybit_prices(body_str, symbols).ok_or(http::Error::Unknown)
	}
}

pub fn parse_bybit_prices(
	price_str: &str,
	symbols: &[String], // 例如 ["BTC", "ETH"]
) -> Option<Vec<(Symbol, Price)>> {
	let root = JSONValue::load(price_str);

	let result = root.get_key_value("result").ok()?;

	let list = result.get_key_value("list").ok()?.iter_array().ok()?;

	let mut out = Vec::new();

	for item in list {
		let sym_val = item.get_key_value("symbol").ok()?;
		let inst_id_str = sym_val.read_string().ok()?; // e.g. "BTCUSDT-PERP"

		if !inst_id_str.ends_with("PERP") {
			continue;
		}
		let symbol_str = &inst_id_str[..inst_id_str.len() - 4]; // e.g. "BTCUSDT"

		if !symbols.iter().any(|s| s == symbol_str) {
			continue;
		}

		let raw_bytes: Vec<u8> = symbol_str.as_bytes().to_vec();

		let bounded_symbol: Symbol = raw_bytes.try_into().ok()?;

		let price_val = item.get_key_value("lastPrice").ok()?;

		let last_str = if price_val.value_type == JSONValueType::String {
			price_val.read_string().ok()?.to_string()
		} else {
			let f = price_val.read_float().ok()?;
			f.to_string()
		};

		let price_cents = parse_price_to_cents(&last_str)?;
		out.push((bounded_symbol, price_cents));
	}

	Some(out)
}
