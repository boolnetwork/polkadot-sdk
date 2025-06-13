#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Vault;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use pallet_assets::Pallet as Assets;
use sp_runtime::{traits::SaturatedConversion, FixedU128};
use sp_std::vec::Vec;

fn mint_usdt<T: Config>(who: &T::AccountId, amount: T::Balance) {
	let owner = account::<T::AccountId>("01", 0, 0);
	let owner_lookup = T::Lookup::unlookup(owner.clone());
	let who_lookup = T::Lookup::unlookup(who.clone());
	let asset_id: <T as pallet_assets::Config>::AssetId = T::UsdtAssetId::get();
	let min_balance: <T as pallet_assets::Config>::Balance = 1u32.into();
	assert_ok!(pallet_assets::Pallet::<T>::force_create(
		frame_system::RawOrigin::Root.into(),
		T::UsdtAssetId::get().into(),
		owner_lookup.clone().into(),
		true,
		min_balance,
	)
	.or_else(|e| if e == pallet_assets::Error::<T>::InUse.into() { Ok(()) } else { Err(e) }));
	assert_ok!(Assets::<T>::mint(
		RawOrigin::Signed(owner.clone()).into(),
		asset_id.clone().into(),
		who_lookup.clone(),
		amount,
	));
}

benchmarks! {
	// Benchmark `stake(origin, amount)`.
	stake {
		let caller: T::AccountId = whitelisted_caller();
		let amount: T::Balance = 1_000_000u32.into();
		mint_usdt::<T>(&caller, amount);
	}: _(RawOrigin::Signed(caller.clone()), amount)
	verify {
		// After staking, total_stake should equal amount.
		assert_eq!(Vault::<T>::total_stake(), amount);
	}

	// Benchmark `add_reward(origin, amount)`.
	add_reward {
		let caller: T::AccountId = whitelisted_caller();
		let stake_amount: T::Balance = 1_000_000u32.into();
		let reward: T::Balance = 500_000u32.into();

		mint_usdt::<T>(&caller, stake_amount + reward);
		// Caller stakes.
		assert_ok!(Vault::<T>::stake(RawOrigin::Signed(caller.clone()).into(), stake_amount));
	}: _(RawOrigin::Signed(caller.clone()), reward)
	verify {
		// acc_reward_per_share = reward / total_stake
		let total = Vault::<T>::total_stake();
		let expect = FixedU128::saturating_from_rational(
			reward.saturated_into::<u128>(),
			total.saturated_into::<u128>(),
		);
		assert_eq!(Vault::<T>::acc_reward_per_share(), expect);
	}

	// Benchmark `claim(origin)`.
	claim {
		let caller: T::AccountId = whitelisted_caller();
		let stake_amount: T::Balance = 1_000_000u32.into();
		let reward: T::Balance = 500_000u32.into();
		mint_usdt::<T>(&caller, stake_amount + reward);

		// Stake then add_reward.
		assert_ok!(Vault::<T>::stake(RawOrigin::Signed(caller.clone()).into(), stake_amount));
		assert_ok!(Vault::<T>::add_reward(RawOrigin::Signed(caller.clone()).into(), reward));
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		// pending_reward should equal 0
		assert_eq!(Vault::<T>::pending_reward(&caller), 0u32.into());
	}

	// // Benchmark `unstake(origin, amount)`.
	unstake {
		let caller: T::AccountId = whitelisted_caller();
		let stake_amount: T::Balance = 1_000_000u32.into();
		mint_usdt::<T>(&caller, stake_amount);
		assert_ok!(Vault::<T>::stake(RawOrigin::Signed(caller.clone()).into(), stake_amount));
		// Advance blocks past cooldown
		let cooldown = T::MinStakeDuration::get();
		frame_system::Pallet::<T>::set_block_number(cooldown + 1u32.into());
		let unstake_amount: T::Balance = (stake_amount / 2u32.into()).into();
	}: _(RawOrigin::Signed(caller.clone()), unstake_amount)
	verify {
		assert_eq!(Vault::<T>::total_stake(), stake_amount - unstake_amount);
	}

	// Benchmark `exit(origin)`.
	exit {
		let caller: T::AccountId = whitelisted_caller();
		let stake_amount: T::Balance = 1_000_000u32.into();
		let reward: T::Balance = 200_000u32.into();
		mint_usdt::<T>(&caller, stake_amount + reward);
		assert_ok!(Vault::<T>::stake(RawOrigin::Signed(caller.clone()).into(), stake_amount));
		assert_ok!(Vault::<T>::add_reward(RawOrigin::Signed(caller.clone()).into(), reward));
		// Advance past cooldown
		let cooldown = T::MinStakeDuration::get();
		frame_system::Pallet::<T>::set_block_number(cooldown + 1u32.into());
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		// After exit, total_stake == 0
		assert_eq!(Vault::<T>::total_stake(), Zero::zero());
	}

	// Benchmark `set_min_stake_duration(origin, new_min)`.
	set_min_stake_duration {
		let new_min = T::MinStakeDuration::get() + 50u32.into();
	}: _(RawOrigin::Root, new_min)
	verify {
		assert_eq!(Vault::<T>::min_stake_duration(), new_min);
	}

	// Benchmark `compensate(origin, to, amount, reason)`.
	compensate {
		let caller: T::AccountId = whitelisted_caller();
		// 1) Set the clearing account.
		assert_ok!(Vault::<T>::set_clearing_account(RawOrigin::Root.into(), caller.clone()));
		// 2) Ensure vault has funds.
		let vault = Vault::<T>::vault_account();
		let seed: T::Balance = 200_000u32.into();

		mint_usdt::<T>(&vault, seed);
		mint_usdt::<T>(&caller, seed);

		// 3) Stake one user so total_stake > 0.
		let staker = account::<T::AccountId>("01", 0, 0);
		mint_usdt::<T>(&staker, 500_000u32.into());
		assert_ok!(Vault::<T>::stake(RawOrigin::Signed(staker.clone()).into(), 500_000u32.into()));
		assert_ok!(Vault::<T>::add_reward(
			RawOrigin::Signed(caller.clone()).into(),
			seed
		));
		let shortfall: T::Balance = 100_000u32.into();
		let reason: Vec<u8> = b"bench".to_vec();
	}: _(RawOrigin::Signed(caller.clone()), staker.clone(), shortfall, reason.clone())
	verify {
		// acc = 0.4 - 0.2 = 0.2
		let acc = Vault::<T>::acc_reward_per_share();
		// let expected = FixedU128::saturating_from_rational(100_000, 500_000); // 0.2
		let expected = FixedU128::saturating_from_rational(2u128, 10u128);
		assert_eq!(acc, expected);
	}

	impl_benchmark_test_suite!(
		Vault,
		crate::mock::new_test_ext(),
		crate::mock::Test
	);
}
