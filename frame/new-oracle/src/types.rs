use alloc::{string::String, vec::Vec};
use frame_support::{sp_runtime::traits::ConstU32, BoundedVec};
use sp_runtime::offchain::http;

pub type Symbol = BoundedVec<u8, ConstU32<32>>;
pub type Price = u128;
pub type PriceList = Vec<(Symbol, Price)>;

pub trait PriceSource: Sync {
	fn name(&self) -> &'static str;
	fn weight(&self) -> u32;
	fn fetch(&self, symbols: &[String]) -> Result<PriceList, http::Error>;
}
