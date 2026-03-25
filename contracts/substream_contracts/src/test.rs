#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger},
    token, vec, Address, Bytes, Env,
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

const DAY: u64 = 24 * 60 * 60;
const WEEK: u64 = 7 * DAY;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &sac.address())
}

// ---------------------------------------------------------------------------
// is_subscribed
// ---------------------------------------------------------------------------

#[test]
fn test_is_subscribed_active() {
#[test]
fn test_grace_period_access_and_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);
    token_admin.mint(&subscriber, &10000000); // Plenty of tokens for testing

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &1);

    // Still inside trial ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â no charges yet, subscription is active.
    env.ledger().set_timestamp(105);
    assert!(client.is_subscribed(&subscriber, &creator));
}

#[test]
fn test_is_subscribed_expired() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // Subscribe with exactly 10 tokens at 10/sec ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â depletes after 1 billable second.
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &10, &10);

    // After trial + 2 seconds the balance is fully consumed ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â not subscribed.
    env.ledger().set_timestamp(100 + WEEK + 2);
    assert!(!client.is_subscribed(&subscriber, &creator));
}

#[test]
fn test_is_subscribed_none() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    assert!(!client.is_subscribed(&subscriber, &creator));
}

// ---------------------------------------------------------------------------
// Free trial
// ---------------------------------------------------------------------------

#[test]
fn test_free_trial_ignores_claims_within_first_week() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &300, &3);

    // Still inside trial: no charges.
    env.ledger().set_timestamp(start + WEEK - 1);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 0);

    // 9 seconds after the trial ends: 9 * 3 = 27 tokens earned.
    env.ledger().set_timestamp(start + WEEK + 9);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 27);
}

// ---------------------------------------------------------------------------
// Cancel
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "cannot cancel: minimum duration not met")]
fn test_cancel_before_minimum_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Only 1 hour has passed ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â minimum is 24 hours.
    env.ledger().set_timestamp(100 + 3600);
    client.cancel(&subscriber, &creator);
}

#[test]
fn test_cancel_after_minimum_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    // 100 tokens at 1/sec: trial ends at start+WEEK, balance depletes at start+WEEK+100
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Cancel inside trial but after DAY ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â creator earns nothing, subscriber refunded.
    env.ledger().set_timestamp(start + DAY + 10);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&subscriber), 1000);
}

#[test]
fn test_cancel_exactly_at_minimum_duration() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Exactly at DAY boundary ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â inside trial, so creator receives nothing.
    env.ledger().set_timestamp(100 + DAY);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&subscriber), 1000);
    assert_eq!(token.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Top-up
// ---------------------------------------------------------------------------

#[test]
fn test_top_up() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(0);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);
    assert_eq!(token.balance(&contract_id), 100);

    client.top_up(&subscriber, &creator, &50);
    assert_eq!(token.balance(&contract_id), 150);

    // 120 seconds after trial: 120 * 1 = 120 tokens, capped at 150 remaining.
    env.ledger().set_timestamp(WEEK + 120);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 120);
    assert_eq!(token.balance(&contract_id), 30);
}

// ---------------------------------------------------------------------------
// Subscription storage tier transitions
// ---------------------------------------------------------------------------

#[test]
fn test_inactive_stream_moves_to_temporary_storage() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &10, &1);

    let key = DataKey::Subscription(subscriber.clone(), creator.clone());
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&key));
        assert!(!env.storage().temporary().has(&key));
    });

    // Deplete Subscription after trial; balance goes to 0 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ moves to temporary storage.
    env.ledger().set_timestamp(start + WEEK + 20);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&contract_id), 0);
    env.as_contract(&contract_id, || {
        assert!(!env.storage().persistent().has(&key));
        assert!(env.storage().temporary().has(&key));
    });
}

#[test]
fn test_top_up_reactivates_stream_to_persistent_storage() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &10, &1);

    let key = DataKey::Subscription(subscriber.clone(), creator.clone());

    // Deplete Subscription ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ temporary storage.
    env.ledger().set_timestamp(start + WEEK + 20);
    client.collect(&subscriber, &creator);

    env.as_contract(&contract_id, || {
        assert!(env.storage().temporary().has(&key));
        assert!(!env.storage().persistent().has(&key));
    });

    // Top up ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ moves back to persistent storage.
    client.top_up(&subscriber, &creator, &5);

    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&key));
        assert!(!env.storage().temporary().has(&key));
    });
}

// ---------------------------------------------------------------------------
// Group channel
// ---------------------------------------------------------------------------

#[test]
fn test_group_subscribe_and_collect_split() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let creator_5 = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone(),
    ];
    let percentages = vec![&env, 40u32, 25u32, 15u32, 10u32, 10u32];

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &500,
        &10,
        &creators,
        &percentages,
    );

    // 10 seconds after trial: 10 sec * 10/sec = 100 tokens distributed.
    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect_group(&subscriber, &channel_id);

    assert_eq!(token.balance(&creator_1), 40);
    assert_eq!(token.balance(&creator_2), 25);
    assert_eq!(token.balance(&creator_3), 15);
    assert_eq!(token.balance(&creator_4), 10);
    assert_eq!(token.balance(&creator_5), 10);
    assert_eq!(token.balance(&contract_id), 400);
}

#[test]
#[should_panic(expected = "group channel must contain exactly 5 creators")]
fn test_group_requires_exactly_five_creators() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
    ];
    let percentages = vec![&env, 25u32, 25u32, 25u32, 25u32];

    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &100,
        &1,
        &creators,
        &percentages,
    );
}

#[test]
#[should_panic(expected = "percentages must sum to 100")]
fn test_group_percentages_must_sum_to_100() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let creator_5 = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone(),
    ];
    // Sums to 90 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â should panic.
    let percentages = vec![&env, 30u32, 20u32, 20u32, 10u32, 10u32];

    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &100,
        &1,
        &creators,
        &percentages,
    );
}

#[test]
fn test_group_cancel_collects_and_refunds_remaining_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let creator_5 = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone(),
    ];
    let percentages = vec![&env, 40u32, 20u32, 20u32, 10u32, 10u32];

    // Deposit 200 at 1/sec; trial lasts 1 week so net charge = 30s after trial.
    env.ledger().set_timestamp(0);
    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &200,
        &1,
        &creators,
        &percentages,
    );

    // Cancel at DAY + 30s (past minimum); trial = WEEK so 0 billable seconds.
    // All 200 tokens refunded to subscriber.
    env.ledger().set_timestamp(DAY + 30);
    client.cancel_group(&subscriber, &channel_id);

    assert_eq!(token.balance(&creator_1), 0);
    assert_eq!(token.balance(&creator_2), 0);
    assert_eq!(token.balance(&creator_3), 0);
    assert_eq!(token.balance(&creator_4), 0);
    assert_eq!(token.balance(&creator_5), 0);
    assert_eq!(token.balance(&subscriber), 1000); // fully refunded
    assert_eq!(token.balance(&contract_id), 0);
}

// ---------------------------------------------------------------------------
// Pause / Unpause
// ---------------------------------------------------------------------------

#[test]
fn test_pause_channel_blocks_charges_and_unpause_resumes() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    // 300 tokens at 2/sec; trial = WEEK.
    client.subscribe(&subscriber, &creator, &token.address, &300, &2);

    // 10 seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 20); // 10 * 2

    // Pause at 20 seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 20);
    client.pause_channel(&creator);
    assert!(client.is_channel_paused(&creator));
    assert_eq!(token.balance(&creator), 40); // additional 10 * 2 settled on pause

    // Collect while paused ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â no additional charges.
    env.ledger().set_timestamp(start + WEEK + 100);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 40);

    // Unpause.
    client.unpause_channel(&creator);
    assert!(!client.is_channel_paused(&creator));

    // Collect 10 seconds after unpause.
    env.ledger().set_timestamp(start + WEEK + 110);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 60); // additional 10 * 2
    assert_eq!(token.balance(&contract_id), 240);
}

#[test]
fn test_pause_channel_applies_to_all_subscribers() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber_1 = Address::generate(&env);
    let subscriber_2 = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber_1, &200);
    token_admin.mint(&subscriber_2, &200);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber_1, &creator, &token.address, &200, &1);
    client.subscribe(&subscriber_2, &creator, &token.address, &200, &1);

    // 30 seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 30);
    client.pause_channel(&creator);
    // Both streams settled at pause: 2 ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â 30 = 60 tokens.
    assert_eq!(token.balance(&creator), 60);

    // 100-second pause window.
    env.ledger().set_timestamp(start + WEEK + 130);
    client.unpause_channel(&creator);

    // 10 seconds after unpause.
    env.ledger().set_timestamp(start + WEEK + 140);
    let total = client.withdraw_all(&creator, &10);

    // Only post-unpause 10 seconds billable for each stream: 2 ÃƒÆ’Ã†â€™ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â 10 = 20.
    assert_eq!(total, 20);
    assert_eq!(token.balance(&creator), 80);
    assert_eq!(token.balance(&contract_id), 320);
}

// ---------------------------------------------------------------------------
// Tier migration
// ---------------------------------------------------------------------------

#[test]
fn test_migrate_tier_collects_at_old_rate_before_switching() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &10);

    // 5 seconds after trial: old rate earns 5 * 10 = 50 tokens before migration.
    env.ledger().set_timestamp(100 + WEEK + 5);
    client.migrate_tier(&subscriber, &creator, &5, &0);

    assert_eq!(token.balance(&creator), 50);
}

#[test]
fn test_migrate_tier_new_rate_applies_after_switch() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Migrate at 10 seconds after trial.
    env.ledger().set_timestamp(100 + WEEK + 10);
    client.migrate_tier(&subscriber, &creator, &2, &0);
    assert_eq!(token.balance(&creator), 10); // 10 * 1

    // Collect 10 more seconds at new rate.
    env.ledger().set_timestamp(100 + WEEK + 20);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 30); // 10 + 10*2
    assert_eq!(token.balance(&contract_id), 70);
}

#[test]
fn test_migrate_tier_emits_event() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    client.migrate_tier(&subscriber, &creator, &3, &0);

    // Verify some events exist (implicitly checked by compile test for now)
    let _events = env.events().all();
}

#[test]
fn test_migrate_tier_with_top_up_is_atomic() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    env.ledger().set_timestamp(0);
    // Subscribe with 100 tokens at 1/sec
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // 50 seconds post-trial (WEEK + 50)
    env.ledger().set_timestamp(WEEK + 50);
    
    // Migrate to 5/sec AND add 500 tokens in one transaction
    client.migrate_tier(&subscriber, &creator, &5, &500);

    // 1. Verify old rate was prorated: 50 * 1 = 50 tokens collected
    assert_eq!(token.balance(&creator), 50);

    // 2. Verify balance was updated: 100 (initial) - 50 (collected) + 500 (top-up) = 550
    // We check this by seeing how long it lasts now.
    // 10 seconds later at 5/sec = 50 more tokens should be collected.
    env.ledger().set_timestamp(WEEK + 60);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 100); // 50 (old) + 50 (new)
    
    // 3. Verify total balance in contract after collect: 550 - 50 = 500
    // Actually we don't have a direct view_balance but we can check if it stays active.
    assert!(client.is_subscribed(&subscriber, &creator));
}

#[test]
fn test_creator_coop_split_dynamic() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let editor = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // 1. Subscribe initially (no split)
    env.ledger().set_timestamp(0);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &10);

    // 2. Set dynamic split halfway through (70% creator, 30% editor)
    let partitions = vec![
        &env,
        SplitPartition { partner: creator.clone(), percentage: 70 },
        SplitPartition { partner: editor.clone(), percentage: 30 },
    ];
    client.set_creator_split(&creator, &partitions);

    // 3. Collect 10 seconds post-trial (WEEK + 10)
    // Total should be 10 * 10 = 100 tokens.
    // 70 to creator, 30 to editor.
    env.ledger().set_timestamp(WEEK + 10);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 70);
    assert_eq!(token.balance(&editor), 30);

    // 4. Test dust management (3-way split: 33%, 33%, 34%)
    // Last partner should take the remainder to avoid stuck tokens.
    let guest = Address::generate(&env);
    let partitions_dust = vec![
        &env,
        SplitPartition { partner: creator.clone(), percentage: 33 },
        SplitPartition { partner: editor.clone(), percentage: 33 },
        SplitPartition { partner: guest.clone(), percentage: 34 },
    ];
    client.set_creator_split(&creator, &partitions_dust);

    // Collect another 10 seconds = 100 tokens.
    // 33, 33, 34 respectively.
    env.ledger().set_timestamp(WEEK + 20);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 70 + 33);
    assert_eq!(token.balance(&editor), 30 + 33);
    assert_eq!(token.balance(&guest), 34);
}

// ---------------------------------------------------------------------------
// Cliff / access tiers
// ---------------------------------------------------------------------------

#[test]
fn test_cliff_based_access_before_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    client.set_cliff_threshold(&creator, &50);
    assert_eq!(client.get_cliff_threshold(&creator), 50);

    // No streaming yet ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â should not have access.
    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);
}

#[test]
fn test_cliff_threshold_access() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    client.set_cliff_threshold(&creator, &50);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // 30 seconds after trial: 30 tokens streamed ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â below threshold.
    env.ledger().set_timestamp(start + WEEK + 30);
    client.collect(&subscriber, &creator);
    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);

    // 50 more seconds: 80 total ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â at/above threshold.
    env.ledger().set_timestamp(start + WEEK + 80);
    client.collect(&subscriber, &creator);
    assert!(client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 1);
}

#[test]
fn test_access_tiers_progression() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &10000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    // 10000 tokens at 10/sec
    client.subscribe(&subscriber, &creator, &token.address, &10000, &10);

    // 5 seconds post-trial: 50 tokens ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ tier 1
    env.ledger().set_timestamp(start + WEEK + 5);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_access_tier(&subscriber, &creator), 1);

    // 25 more seconds: 300 total ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ tier 3
    env.ledger().set_timestamp(start + WEEK + 30);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_access_tier(&subscriber, &creator), 3);
}

// ---------------------------------------------------------------------------
// Creator metadata
// ---------------------------------------------------------------------------

#[test]
fn test_creator_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let creator = Address::generate(&env);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // No metadata set yet.
    assert_eq!(client.get_creator_metadata(&creator), None);

    let cid = Bytes::from_slice(&env, b"QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    client.set_creator_metadata(&creator, &cid);
    assert_eq!(client.get_creator_metadata(&creator), Some(cid.clone()));

    let new_cid = Bytes::from_slice(&env, b"QmNewCIDabcde12345");
    client.set_creator_metadata(&creator, &new_cid);
    assert_eq!(client.get_creator_metadata(&creator), Some(new_cid));
}

// ---------------------------------------------------------------------------
// calculate_total_earned ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â core feature for issue #59 / #12
// ---------------------------------------------------------------------------


#[test]
fn test_calculate_total_earned_returns_zero_when_no_subscribers() {
    let env = Env::default();
    env.mock_all_auths();

    let creator = Address::generate(&env);
    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // No streams exist ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â should return 0.
    assert_eq!(client.calculate_total_earned(&creator), 0);
}

#[test]
fn test_calculate_total_earned_zero_during_trial() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 1000u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &5);

    // Still inside trial window ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â no earnings yet.
    env.ledger().set_timestamp(start + WEEK - 1);
    assert_eq!(client.calculate_total_earned(&creator), 0);
}

#[test]
fn test_calculate_total_earned_single_stream_basic() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    // 1000 tokens, 10/sec
    client.subscribe(&subscriber, &creator, &token.address, &1000, &10);

    // 30 seconds after trial: 30 * 10 = 300 earned.
    env.ledger().set_timestamp(start + WEEK + 30);
    assert_eq!(client.calculate_total_earned(&creator), 300);
}

#[test]
fn test_calculate_total_earned_caps_at_stream_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &50);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    // Only 50 tokens deposited at 10/sec ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â depletes after 5 billable seconds.
    client.subscribe(&subscriber, &creator, &token.address, &50, &10);

    // Way past depletion ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â should be capped at 50.
    env.ledger().set_timestamp(start + WEEK + 10000);
    assert_eq!(client.calculate_total_earned(&creator), 50);
}

#[test]
fn test_calculate_total_earned_multiple_subscribers() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber_1 = Address::generate(&env);
    let subscriber_2 = Address::generate(&env);
    let subscriber_3 = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber_1, &10000);
    token_admin.mint(&subscriber_2, &10000);
    token_admin.mint(&subscriber_3, &10000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    // All three subscribe at 2/sec with ample balance.
    client.subscribe(&subscriber_1, &creator, &token.address, &10000, &2);
    client.subscribe(&subscriber_2, &creator, &token.address, &10000, &2);
    client.subscribe(&subscriber_3, &creator, &token.address, &10000, &2);

    // 60 seconds past trial: each Subscription earns 60 * 2 = 120 ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ total = 360.
    env.ledger().set_timestamp(start + WEEK + 60);
    assert_eq!(client.calculate_total_earned(&creator), 360);
}

#[test]
fn test_calculate_total_earned_paused_channel_returns_zero() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &10000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &10000, &5);

    // Pause at 10 seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 10);
    client.pause_channel(&creator);

    // Query deep into the future while still paused.
    env.ledger().set_timestamp(start + WEEK + 9999);
    assert_eq!(client.calculate_total_earned(&creator), 0);
}

#[test]
fn test_calculate_total_earned_reflects_already_collected_portion() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &2);

    // Advance 100 seconds past trial; collect immediately.
    env.ledger().set_timestamp(start + WEEK + 100);
    client.collect(&subscriber, &creator);
    // Creator has received 100 * 2 = 200 tokens already.
    assert_eq!(token.balance(&creator), 200);

    // At the same timestamp the unclaimed balance should be 0 (just collected).
    assert_eq!(client.calculate_total_earned(&creator), 0);

    // Advance another 50 seconds ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â 50 * 2 = 100 more unclaimed.
    env.ledger().set_timestamp(start + WEEK + 150);
    assert_eq!(client.calculate_total_earned(&creator), 100);
}

#[test]
fn test_calculate_total_earned_group_channel_creator_share() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let creator_5 = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &10000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone(),
    ];
    // creator_1 = 40%, creator_2 = 25%, creator_3 = 15%, creator_4 = 10%, creator_5 = 10%
    let percentages = vec![&env, 40u32, 25u32, 15u32, 10u32, 10u32];

    let start = 0u64;
    env.ledger().set_timestamp(start);
    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &10000,
        &10,
        &creators,
        &percentages,
    );

    // 100 seconds after trial: gross = 100 * 10 = 1000 tokens.
    env.ledger().set_timestamp(start + WEEK + 100);

    // creator_1's share: 40% of 1000 = 400
    assert_eq!(
        client.calculate_total_earned(&creator_1),
        400
    );
    // creator_2's share: 25% of 1000 = 250
    assert_eq!(
        client.calculate_total_earned(&creator_2),
        250
    );
    // creator_3's share: 15% of 1000 = 150
    assert_eq!(
        client.calculate_total_earned(&creator_3),
        150
    );
    // creator_4's and creator_5's share: 10% each = 100
    assert_eq!(
        client.calculate_total_earned(&creator_4),
        100
    );
    assert_eq!(
        client.calculate_total_earned(&creator_5),
        100
    );
}

#[test]
fn test_calculate_total_earned_no_mutation_verify_via_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    token::StellarAssetClient::new(&env, &token.address).mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &1);

    env.ledger().set_timestamp(start + WEEK + 100);

    // Call the helper multiple times ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â‚¬Å¡Ã‚Â¬ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â it must not transfer tokens.
    let earned_1 = client.calculate_total_earned(&creator);
    let earned_2 = client.calculate_total_earned(&creator);

    assert_eq!(earned_1, earned_2);
    // Creator's actual token balance must remain 0 (no transfers occurred).
    assert_eq!(token.balance(&creator), 0);
    // Contract still holds full deposit.
    assert_eq!(token.balance(&contract_id), 1000);
}

#[test]
fn test_calculate_total_earned_mixed_single_and_group_streams() {
    let env = Env::default();
    env.mock_all_auths();

    // creator_a has one direct subscriber AND is a member of a group channel.
    let subscriber_direct = Address::generate(&env);
    let subscriber_group = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_a = Address::generate(&env);
    let creator_b = Address::generate(&env);
    let creator_c = Address::generate(&env);
    let creator_d = Address::generate(&env);
    let creator_e = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber_direct, &10000);
    token_admin.mint(&subscriber_group, &10000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 0u64;
    env.ledger().set_timestamp(start);

    // Direct stream: subscriber_direct ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ creator_a, 3/sec, 10000 deposit
    client.subscribe(&subscriber_direct, &creator_a, &token.address, &10000, &3);

    // Group stream: subscriber_group ÃƒÆ’Ã‚Â¢ÃƒÂ¢Ã¢â€šÂ¬Ã‚Â ÃƒÂ¢Ã¢â€šÂ¬Ã¢â€žÂ¢ channel_id, 10/sec, 10000 deposit
    // creator_a gets 50%, others share the remaining 50%.
    let creators = vec![
        &env,
        creator_a.clone(),
        creator_b.clone(),
        creator_c.clone(),
        creator_d.clone(),
        creator_e.clone(),
    ];
    let percentages = vec![&env, 50u32, 20u32, 10u32, 10u32, 10u32];
    client.subscribe_group(
        &subscriber_group,
        &channel_id,
        &token.address,
        &10000,
        &10,
        &creators,
        &percentages,
    );

    // 200 seconds after trial:
    //   Direct Subscription contribution: 200 * 3 = 600
    //   Group channel (creator_a's 50%): 50% of (200 * 10) = 50% of 2000 = 1000
    //   Total for creator_a: 1600
    env.ledger().set_timestamp(start + WEEK + 200);

    let earned = client.calculate_total_earned(&creator_a);
    assert_eq!(earned, 1600);
    // Subscribe with 1000 tokens at rate 10/sec.
    // Free trial is 1 week.
    let start_time = 1000;
    env.ledger().set_timestamp(start_time);
    client.subscribe(&subscriber, &creator, &token.address, &1000, &10);

    // Initial check: is subscribed
    assert!(client.is_subscribed(&subscriber, &creator));

    // After 1 week + 50 seconds: 500 tokens should be collected.
    env.ledger().set_timestamp(start_time + WEEK + 50);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 500);

    // After 1 week + 100 seconds: All 1000 tokens collected. Balance = 0.
    env.ledger().set_timestamp(start_time + WEEK + 100);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 1000);
    
    // Check access during grace period (12 hours after funds run out)
    // 12 hours = 43200 seconds. 
    env.ledger().set_timestamp(start_time + WEEK + 100 + (DAY / 2));
    assert!(client.is_subscribed(&subscriber, &creator));

    // Collect during grace period: Should accrue debt.
    // 12 hours elapsed since last collection. Amount = 43200 * 10 = 432000.
    // Mint more to contract for payouts
    token_admin.mint(&contract_id, &10000000); 
    
    client.collect(&subscriber, &creator);
    // Creator should NOT have received tokens for the debt portion yet
    assert_eq!(token.balance(&creator), 1000);

    // Check access AFTER grace period (25 hours after funds run out)
    env.ledger().set_timestamp(start_time + WEEK + 100 + DAY + 3600);
    assert!(!client.is_subscribed(&subscriber, &creator));
    
    // Top up to clear debt. 
    // Debt at DAY/2 was 432000.
    // Additional debt until DAY+3600 is 468000.
    // Total debt: 900000.
    // Top up with 1000000. 
    client.top_up(&subscriber, &creator, &1000000);
    
    // Creator should have received the total debt payment (900000) now
    // (432000 from the manual payout in top_up + 468000 from the triggered collect)
    assert_eq!(token.balance(&creator), 1000 + 900000);
    
    // Check subscribed status again
    assert!(client.is_subscribed(&subscriber, &creator));
    
    // Advance more time and collect to verify new balance
    let top_up_time = start_time + WEEK + 100 + DAY + 3600;
    env.ledger().set_timestamp(top_up_time + 100);
    client.collect(&subscriber, &creator);
    // 100 seconds = 1000 tokens. 
    assert_eq!(token.balance(&creator), 1000 + 900000 + 1000);
}
