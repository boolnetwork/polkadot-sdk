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
use log::info;
use sp_runtime::offchain::{http, Duration, Timestamp};

pub struct Binance;

impl PriceSource for Binance {
	fn name(&self) -> &'static str {
		"Binance"
	}
	fn weight(&self) -> u32 {
		90
	}
	fn fetch(&self, symbols: &[String]) -> Result<PriceList, http::Error> {
		// let b_url = "https://fapi.binance.com/fapi/v1/ticker/price";
		let b_url = "http://localhost:3000/binance"; //local test api

		let deadline: sp_core::offchain::Timestamp =
			sp_io::offchain::timestamp().add(Duration::from_millis(10_000));
		let request = http::Request::get(&b_url);

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

		parse_binance_prices(body_str, symbols).ok_or(http::Error::Unknown)
	}
}

pub fn parse_binance_prices(
	price_str: &str,
	symbols: &[String], // 例如 ["BTC", "ETH"]
) -> Option<Vec<(Symbol, Price)>> {
	// 1. 先用 lite_json 解析顶层 JSON
	let val = lite_json::parse_json(price_str).ok()?;
	let items = match val {
		JsonValue::Array(arr) => arr,
		_ => return None,
	};

	let mut out = Vec::new();

	for item in items {
		let inner = match item {
			JsonValue::Object(o) => o,
			_ => continue,
		};

		// 取 instId
		let inst_id_chars = inner
			.iter()
			.find(|(k, _)| k.iter().copied().eq("symbol".chars()))
			.and_then(|(_, v)| if let JsonValue::String(s) = v { Some(s) } else { None })?;
		let inst_id_str: String = inst_id_chars.into_iter().collect();

		if !inst_id_str.ends_with("USDT") {
			continue;
		}
		// 去掉尾部 "USDT"
		let symbol_str = &inst_id_str[..inst_id_str.len() - 4];

		// **新增：如果传入的 symbols 列表里不包含，就跳过**
		if !symbols.iter().any(|s| s == symbol_str) {
			continue;
		}

		let raw_bytes: Vec<u8> = symbol_str.as_bytes().to_vec();
		let bounded_symbol: Symbol = raw_bytes.try_into().ok()?;

		// 取 last 字段并转换成 u128 分为单位
		let last_val = inner
			.iter()
			.find(|(k, _)| k.iter().copied().eq("price".chars()))
			.map(|(_, v)| v)?;
		let last_str: String = match last_val {
			JsonValue::String(chars) => chars.into_iter().collect(),
			JsonValue::Number(n) => {
				let mut s = n.integer.to_string();
				if n.fraction_length > 0 {
					let mut frac = n.fraction.to_string();
					while frac.len() < n.fraction_length as usize {
						frac = format!("0{}", frac);
					}
					s.push('.');
					s.push_str(&frac);
				}
				s
			},
			_ => continue,
		};

		let price_cents = parse_price_to_cents(&last_str)?;

		out.push((bounded_symbol, price_cents));
	}

	Some(out)
}
