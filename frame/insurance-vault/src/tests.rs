#![cfg(test)]
use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use frame_system::Origin;

fn mint_usdt(account_mint: &AccountIdOf<Test>, amount: u128) -> DispatchResult {
	let owner = account(1);
	pallet_assets::Pallet::<Test>::mint(
		RuntimeOrigin::signed(owner.clone()),
		UsdtId::get(),
		account_mint.clone(),
		amount,
	)?;

	Ok(())
}

#[test]
fn set_min_stake_duration_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(InsuranceVault::min_stake_duration(), 0);

		let new_duration: u64 = 200;
		assert_ok!(InsuranceVault::set_min_stake_duration(RuntimeOrigin::root(), new_duration));

		assert_eq!(InsuranceVault::min_stake_duration(), new_duration);
	});
}

#[test]
fn stake_then_unstake_after_cooldown() {
	new_test_ext().execute_with(|| {
		let bob = account(2);
		let stake_amount = 500_000u128;
		mint_usdt(&bob, stake_amount);
		InsuranceVault::set_min_stake_duration(RuntimeOrigin::root(), 200);

		// Before staking, total_stake == 0
		assert_eq!(InsuranceVault::total_stake(), 0);

		// bob stakes
		assert_ok!(InsuranceVault::stake(RuntimeOrigin::signed(bob.clone()), stake_amount));
		assert_eq!(InsuranceVault::total_stake(), stake_amount);

		// Try immediate unstake (should fail due to cooldown)
		assert_noop!(
			InsuranceVault::unstake(RuntimeOrigin::signed(bob.clone()), stake_amount / 2),
			Error::<Test>::CooldownNotPassed
		);

		// Fast-forward blocks to pass cooldown
		System::set_block_number(
			System::block_number().saturating_add(MinStakeDuration::<Test>::get()),
		);

		// Now unstake half
		assert_ok!(InsuranceVault::unstake(RuntimeOrigin::signed(bob), stake_amount / 2));
		assert_eq!(InsuranceVault::total_stake(), stake_amount / 2);
	});
}

#[test]
fn add_reward_and_claim_distribution() {
	new_test_ext().execute_with(|| {
		let alice = account(1);
		let bob = account(2);

		mint_usdt(&bob, 200_000);

		// Alice stakes all 200k
		assert_ok!(InsuranceVault::stake(RuntimeOrigin::signed(bob.clone()), 200_000));
		assert_eq!(InsuranceVault::total_stake(), 200_000);

		// Add 100k reward
		assert_ok!(InsuranceVault::add_reward(RuntimeOrigin::signed(alice.clone()), 100_000));
		// acc_reward_per_share = 100k / 200k = 0.5
		let acc = InsuranceVault::acc_reward_per_share();
		assert_eq!(acc, FixedU128::saturating_from_rational(100_000, 200_000));

		// Claim: Alice should get 100k
		assert_ok!(InsuranceVault::claim(RuntimeOrigin::signed(bob.clone())));
		// Her balance (in USDT asset) increased by 100k
		assert_eq!(Assets::balance(UsdtId::get(), bob.clone()), 100_000);
	})
}

#[test]
fn exit_returns_principal_and_reward() {
	new_test_ext().execute_with(|| {
		let alice = account(1);
		let bob = account(2);
		mint_usdt(&bob, 300_000);

		// Bob stakes 300k
		assert_ok!(InsuranceVault::stake(RuntimeOrigin::signed(bob.clone()), 300_000));
		// add 150k reward
		assert_ok!(InsuranceVault::add_reward(RuntimeOrigin::signed(alice), 150_000));

		// fast forward past cooldown
		System::set_block_number(
			System::block_number().saturating_add(MinStakeDuration::<Test>::get()),
		);

		// exit should return 300k + 150k
		assert_ok!(InsuranceVault::exit(RuntimeOrigin::signed(bob.clone())));
		assert_eq!(Assets::balance(UsdtId::get(), bob.clone()), 450_000);
		// total_stake resets to zero
		assert_eq!(InsuranceVault::total_stake(), 0);
	})
}

#[test]
fn compensate_transfers_and_reduces_acc() {
	new_test_ext().execute_with(|| {
		let vault = InsuranceVault::vault_account();
		let clear = account(9);
		let account_3 = account(3);
		mint_usdt(&account_3, 400_000);
		// set clearing account
		assert_ok!(InsuranceVault::set_clearing_account(RuntimeOrigin::root(), clear.clone()));
		// mint some USDT into vault so it can pay
		mint_usdt(&vault, 100_000);
		// stake so total_stake > 0
		assert_ok!(InsuranceVault::stake(RuntimeOrigin::signed(account_3), 400_000));
		// remember old acc
		let old_acc = InsuranceVault::acc_reward_per_share();

		// compensate shortfall = 50k
		assert_ok!(InsuranceVault::compensate(
			RuntimeOrigin::signed(clear.clone()),
			clear.clone(),
			50_000,
			b"test shortfall".to_vec()
		));

		// clearing account balance +50k
		assert_eq!(Assets::balance(UsdtId::get(), clear.clone()), 50_000);
		// acc_reward_per_share dropped by 50k/200k = 0.25
		let new_acc = InsuranceVault::acc_reward_per_share();
		assert_eq!(
			new_acc,
			old_acc.saturating_sub(FixedU128::saturating_from_rational(50_000, 200_000))
		);
	})
}
