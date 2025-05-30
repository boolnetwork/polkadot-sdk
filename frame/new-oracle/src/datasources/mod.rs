use crate::types::PriceSource;
use alloc::vec::Vec;
use alloc::vec;

#[cfg(feature = "cryptocompare")]
pub mod cryptocompare;

#[cfg(feature = "cryptocompare")]
use cryptocompare::CryptoCompare;

#[cfg(feature = "okx")]
pub mod okx;

#[cfg(feature = "okx")]
use okx::OKX;

#[cfg(feature = "binance")]
pub mod binance;

#[cfg(feature = "binance")]
use binance::Binance;

#[cfg(feature = "coinbase")]
pub mod coinbase;

#[cfg(feature = "coinbase")]
use coinbase::Coinbase;

#[cfg(feature = "bybit")]
pub mod bybit;

#[cfg(feature = "bybit")]
use bybit::Bybit;



pub fn active_sources() -> Vec<&'static dyn PriceSource> {
    let mut list: Vec<&'static dyn PriceSource> = vec![];

    #[cfg(feature = "cryptocompare")]
    list.push(&CryptoCompare);

    #[cfg(feature = "okx")]
    list.push(&OKX);

    #[cfg(feature = "binance")]
    list.push(&Binance);

    #[cfg(feature = "coinbase")]
    list.push(&Coinbase);

    #[cfg(feature = "bybit")]
    list.push(&Bybit);

    list
}

pub fn parse_price_to_cents(price: &str) -> Option<u128> {
    let parts: Vec<&str> = price.split('.').collect();
    let int_part: u128 = parts[0].parse().ok()?;
    Some(if parts.len() == 1 {
        int_part * 100
    } else {
        let frac = parts[1];
        let frac_val: u128 = frac.parse().ok()?;
        let fl = frac.len() as u32;
        if fl >= 2 {
            int_part * 100 + frac_val / 10u128.pow(fl - 2)
        } else {
            int_part * 100 + frac_val * 10u128.pow(2 - fl)
        }
    })
}

