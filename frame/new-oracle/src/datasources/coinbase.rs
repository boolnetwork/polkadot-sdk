use crate::types::{PriceSource, PriceList, Symbol, Price};
use sp_runtime::offchain::{http, Duration, Timestamp};
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use alloc::format;
use core::str::from_utf8;
use lite_json::json::JsonValue;
use frame_support::BoundedVec;
use frame_support::traits::ConstU32;
use super::parse_price_to_cents;

pub struct Coinbase;

impl PriceSource for Coinbase {
    fn name(&self) -> &'static str {
        "Coinbase"
    }
    fn weight(&self) -> u32 {60}

    fn fetch(&self, symbols: &[String]) -> Result<PriceList, http::Error> {
        let mut out = Vec::new();
        let deadline: sp_core::offchain::Timestamp = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
        for symbol in symbols {
            // "https://api.exchange.coinbase.com/products/{}-USDT/ticker",
            let url = format!(
                "http://localhost:3000/coinbase?symbol={}-USDT",
                symbol
            );

            let request = http::Request::get(&url)
                .add_header("User-Agent", "Substrate-Offchain-Worker")
                .add_header("Accept", "application/json")
                .deadline(deadline)
                .send()
                .map_err(|_| http::Error::IoError)?;

            let response = request
                .try_wait(deadline)
                .map_err(|_| http::Error::DeadlineReached)??;

            let body = response.body().collect::<Vec<u8>>();
            let body_str = sp_std::str::from_utf8(&body).map_err(|_| http::Error::Unknown)?;

            if let Some(prices) = parse_coinbase_price(body_str, symbol) {
                out.push(prices);
            }
        }

        Ok(out)
    }

}

pub fn parse_coinbase_price(body_str: &str, symbol: &str) -> Option<(Symbol, Price)> {
    let val = lite_json::parse_json(body_str).ok()?;
    let obj = match val {
        JsonValue::Object(o) => o,
        _ => return None,
    };

    // 取 price 字段
    let price_val = obj
        .into_iter()
        .find(|(k, _)| k.iter().copied().eq("price".chars()))
        .map(|(_, v)| v)?;

    let price_str: String = match price_val {
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
        }
        _ => return None,
    };

    let price_cents = parse_price_to_cents(&price_str)?;

    let raw_bytes: Vec<u8> = symbol.as_bytes().to_vec();
    let bounded_symbol: Symbol = raw_bytes.try_into().ok()?;

    Some((bounded_symbol, price_cents))
}
