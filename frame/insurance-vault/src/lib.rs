#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

pub use pallet::*;

use frame_support::{pallet_prelude::*, traits::Get, PalletId};

// use sp_arithmetic::FixedU128;

use frame_system::RawOrigin;
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, StaticLookup, Zero},
	FixedPointNumber, FixedU128, Saturating,
};
use sp_std::{convert::TryInto, vec::Vec};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use pallet_assets::Pallet as Assets;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		#[pallet::constant]
		type UsdtAssetId: Get<Self::AssetId>;

		#[pallet::constant]
		type MinStakeDuration: Get<Self::BlockNumber>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		type WeightInfo: WeightInfo;
	}

	/// Type alias for the USDT balance (inherited from pallet_assets).
	pub type BalanceOf<T> = <T as pallet_assets::Config>::Balance;
	/// Type alias for block numbers.
	pub type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

	// ─────────────────────────────────────────────────────────────────────────────
	// STORAGE ITEMS
	// ─────────────────────────────────────────────────────────────────────────────

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn clearing_account)]
	pub type ClearingAccount<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_stake)]
	pub type UserStake<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn reward_debt)]
	pub type RewardDebt<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_reward)]
	pub type PendingReward<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn acc_reward_per_share)]
	pub type AccRewardPerShare<T: Config> = StorageValue<_, FixedU128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn stake_timestamp)]
	pub type StakeTimestamp<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn min_stake_duration)]
	pub type MinStakeDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Staked(T::AccountId, BalanceOf<T>),
		Unstaked(T::AccountId, BalanceOf<T>),
		RewardClaimed(T::AccountId, BalanceOf<T>),
		Exited(T::AccountId, BalanceOf<T>, BalanceOf<T>),
		RewardAdded(T::AccountId, BalanceOf<T>),
		MinStakeDurationSet(BlockNumberOf<T>),
		ClearingAccountSet(T::AccountId),
		Compensated(T::AccountId, BalanceOf<T>, Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {
		ZeroAmount,
		InsufficientBalance,
		NoStake,
		UnstakeExceedsStake,
		CooldownNotPassed,
		NoReward,
		NoPoolStake,
		NoClearingAccount,
		NotClearingAccount,
	}

	// ─────────────────────────────────────────────────────────────────────────────
	// DISPATCHABLES
	// ─────────────────────────────────────────────────────────────────────────────

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::stake())]
		pub fn stake(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

			Self::settle_reward(&who)?;

			let vault: T::AccountId = Self::vault_account();

			let dest: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(vault.clone());

			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();

			Assets::<T>::transfer(
				RawOrigin::Signed(who.clone()).into(),
				asset_id_param,
				dest,
				amount,
			)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

			let new_stake = Self::user_stake(&who).saturating_add(amount);
			UserStake::<T>::insert(&who, new_stake);
			TotalStake::<T>::mutate(|v| *v = v.saturating_add(amount));

			let acc: FixedU128 = Self::acc_reward_per_share();
			let stake_u128: u128 = new_stake.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let accrued_int: u128 =
				acc.checked_mul_int(stake_u128).ok_or(Error::<T>::NoPoolStake)?;
			let new_debt: BalanceOf<T> =
				accrued_int.try_into().map_err(|_| Error::<T>::NoPoolStake)?;

			RewardDebt::<T>::insert(&who, new_debt);

			let current_block = <frame_system::Pallet<T>>::block_number();

			StakeTimestamp::<T>::insert(&who, current_block);
			Self::deposit_event(Event::<T>::Staked(who.clone(), amount));

			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::unstake())]
		pub fn unstake(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

			let user_staked: BalanceOf<T> = Self::user_stake(&who);
			ensure!(user_staked > Zero::zero(), Error::<T>::NoStake);
			ensure!(amount <= user_staked, Error::<T>::UnstakeExceedsStake);

			let last_stake_block = Self::stake_timestamp(&who);
			let now_block = <frame_system::Pallet<T>>::block_number();
			let min_duration = T::MinStakeDuration::get();
			ensure!(
				now_block.saturating_sub(last_stake_block) >= min_duration,
				Error::<T>::CooldownNotPassed
			);

			Self::settle_reward(&who)?;

			let new_stake = user_staked.saturating_sub(amount);
			UserStake::<T>::insert(&who, new_stake);
			TotalStake::<T>::mutate(|v| *v = v.saturating_sub(amount));

			let acc: FixedU128 = Self::acc_reward_per_share();
			let new_stake_u128: u128 = new_stake.try_into().map_err(|_| Error::<T>::NoPoolStake)?;

			let accrued_int: u128 =
				acc.checked_mul_int(new_stake_u128).ok_or(Error::<T>::NoPoolStake)?;

			let new_debt: BalanceOf<T> =
				accrued_int.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			RewardDebt::<T>::insert(&who, new_debt);

			let vault: T::AccountId = Self::vault_account();
			let dest_unlookup: <T::Lookup as StaticLookup>::Source =
				T::Lookup::unlookup(who.clone());
			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();

			Assets::<T>::transfer(
				RawOrigin::Signed(vault.clone()).into(),
				asset_id_param,
				dest_unlookup,
				amount,
			)
			.map_err(|_| Error::<T>::NoStake)?;

			Self::deposit_event(Event::<T>::Unstaked(who.clone(), amount));

			Ok(().into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::claim())]
		pub fn claim(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			Self::settle_reward(&who);

			let reward: BalanceOf<T> = Self::pending_reward(&who);
			ensure!(reward > Zero::zero(), Error::<T>::NoReward);

			PendingReward::<T>::insert(&who, BalanceOf::<T>::zero());

			let vault: T::AccountId = Self::vault_account();
			let dest_unlookup: <T::Lookup as StaticLookup>::Source =
				T::Lookup::unlookup(who.clone());
			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();

			Assets::<T>::transfer(
				RawOrigin::Signed(vault.clone()).into(),
				asset_id_param,
				dest_unlookup,
				reward,
			)
			.map_err(|_| Error::<T>::NoReward)?;

			Self::deposit_event(Event::<T>::RewardClaimed(who.clone(), reward));

			Ok(().into())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::exit())]
		pub fn exit(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let user_staked: BalanceOf<T> = Self::user_stake(&who);
			ensure!(user_staked > Zero::zero(), Error::<T>::NoReward);

			let last_stake_block = Self::stake_timestamp(&who);
			let now_block = <frame_system::Pallet<T>>::block_number();
			let min_duration = Self::min_stake_duration();
			ensure!(
				now_block.saturating_sub(last_stake_block) >= min_duration,
				Error::<T>::CooldownNotPassed
			);

			Self::settle_reward(&who);
			let reward: BalanceOf<T> = Self::pending_reward(&who);

			PendingReward::<T>::insert(&who, BalanceOf::<T>::zero());
			RewardDebt::<T>::insert(&who, BalanceOf::<T>::zero());
			StakeTimestamp::<T>::remove(&who);

			TotalStake::<T>::mutate(|v| *v = v.saturating_sub(user_staked));

			let vault = Self::vault_account();
			let dest_unlookup: <T::Lookup as StaticLookup>::Source =
				T::Lookup::unlookup(who.clone());
			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();

			Assets::<T>::transfer(
				RawOrigin::Signed(vault.clone()).into(),
				asset_id_param.clone(),
				dest_unlookup.clone(),
				user_staked,
			)
			.map_err(|_| Error::<T>::NoStake)?;

			if reward > Zero::zero() {
				Assets::<T>::transfer(
					RawOrigin::Signed(vault.clone()).into(),
					asset_id_param,
					dest_unlookup,
					reward,
				)
				.map_err(|_| Error::<T>::NoReward)?;
			}

			Self::deposit_event(Event::<T>::Exited(who.clone(), user_staked, reward));

			Ok(().into())
		}

		#[pallet::call_index(5)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::add_reward())]
		pub fn add_reward(
			origin: OriginFor<T>,
			amount: BalanceOf<T>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			ensure!(amount > Zero::zero(), Error::<T>::ZeroAmount);

			let vault = Self::vault_account();
			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();
			let dest_unlookup: <T::Lookup as StaticLookup>::Source =
				T::Lookup::unlookup(vault.clone());

			Assets::<T>::transfer(
				RawOrigin::Signed(who.clone()).into(),
				asset_id_param,
				dest_unlookup,
				amount,
			)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

			let total: BalanceOf<T> = Self::total_stake();
			ensure!(total > Zero::zero(), Error::<T>::NoPoolStake);
			let total_u128: u128 = total.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let amount_u128: u128 = amount.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let delta = FixedU128::saturating_from_rational(amount_u128, total_u128);

			AccRewardPerShare::<T>::mutate(|v| *v = v.saturating_add(delta));
			Self::deposit_event(Event::<T>::RewardAdded(who.clone(), amount));

			Ok(().into())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_min_stake_duration())]
		pub fn set_min_stake_duration(
			origin: OriginFor<T>,
			new: BlockNumberOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			MinStakeDuration::<T>::put(new);
			Self::deposit_event(Event::MinStakeDurationSet(new));
			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(10_000)]
		pub fn set_clearing_account(
			origin: OriginFor<T>,
			acct: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			ClearingAccount::<T>::put(&acct);
			Self::deposit_event(Event::ClearingAccountSet(acct));
			Ok(().into())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::compensate())]
		pub fn compensate(
			origin: OriginFor<T>,
			to: T::AccountId,
			shortfall: BalanceOf<T>,
			reason: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let cla = ClearingAccount::<T>::get().ok_or(Error::<T>::NoClearingAccount)?;
			ensure!(who == cla, Error::<T>::NotClearingAccount);

			let vault = Self::vault_account();
			let dest: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(to.clone());
			let asset_id_param: T::AssetIdParameter = T::UsdtAssetId::get().into();
			Assets::<T>::transfer(
				RawOrigin::Signed(vault.clone()).into(),
				asset_id_param,
				dest,
				shortfall,
			)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

			let total = Self::total_stake();
			ensure!(total > Zero::zero(), Error::<T>::NoPoolStake);
			let total_u128: u128 = total.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let short_u128: u128 = shortfall.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let delta = FixedU128::saturating_from_rational(short_u128, total_u128);

			AccRewardPerShare::<T>::mutate(|acc| *acc = acc.saturating_sub(delta));

			Self::deposit_event(Event::<T>::Compensated(to, shortfall, reason));
			Ok(().into())
		}
	}

	// ─────────────────────────────────────────────────────────────────────────────
	// INTERNAL HELPERS
	// ─────────────────────────────────────────────────────────────────────────────

	impl<T: Config> Pallet<T> {
		pub fn vault_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
		pub fn settle_reward(who: &T::AccountId) -> DispatchResult {
			let stake: BalanceOf<T> = Self::user_stake(who);
			if stake.is_zero() {
				return Ok(());
			}
			let acc = Self::acc_reward_per_share();
			let stake_u128: u128 = stake.try_into().map_err(|_| Error::<T>::NoPoolStake)?;
			let accrued_u128: u128 =
				acc.checked_mul_int(stake_u128).ok_or(Error::<T>::NoPoolStake)?;

			let prev_debt: BalanceOf<T> = RewardDebt::<T>::get(who);
			let prev_debt_128: u128 = prev_debt.try_into().map_err(|_| Error::<T>::NoPoolStake)?;

			let reward_128: u128 = accrued_u128.saturating_sub(prev_debt_128);
			let reward: BalanceOf<T> =
				reward_128.try_into().map_err(|_| Error::<T>::NoPoolStake)?;

			if reward > <T as pallet_assets::Config>::Balance::zero() {
				PendingReward::<T>::mutate(who, |p| *p = p.saturating_add(reward));
			}

			let new_debt: BalanceOf<T> =
				accrued_u128.try_into().map_err(|_| Error::<T>::NoPoolStake)?;

			RewardDebt::<T>::insert(who, new_debt);

			Ok(())
		}
	}
}
