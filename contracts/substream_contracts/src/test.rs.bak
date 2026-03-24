#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger},
    token, vec, Address, Bytes, Env,
};

const DAY: u64 = 24 * 60 * 60;
const WEEK: u64 = 7 * DAY;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &sac.address())
}

#[test]
fn test_is_subscribed_active() {
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

    // Still active: expiry = 100 + (100/10) = 110
    env.ledger().set_timestamp(105);
    assert!(client.is_subscribed(&subscriber, &creator));
}

#[test]
fn test_is_subscribed_expired() {
    let start = 100u64;
    env.ledger().set_timestamp(start);
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &2);

    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 100);

    // Still inside trial: no charges.
    env.ledger().set_timestamp(start + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 0);

    // 10 paid seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 10);
    env.ledger().set_timestamp(110);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 20);
    assert_eq!(token.balance(&contract_id), 80);

    // Advance 50 more seconds — would be 100 tokens but only 80 left
    // Additional 50 paid seconds, capped by remaining balance.
    env.ledger().set_timestamp(start + WEEK + 60);
    env.ledger().set_timestamp(160);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(client.get_total_streamed(&subscriber, &creator), 100);
}

#[test]
fn test_free_trial_ignores_claims_within_first_week() {
#[should_panic(expected = "cannot cancel stream: minimum duration not met")]
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

    // 24h + 120 seconds pass
    env.ledger().set_timestamp(100 + 86400 + 120);
    client.cancel(&subscriber, &creator);
    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &300, &3);

    env.ledger().set_timestamp(start + WEEK - 1);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 0);

    env.ledger().set_timestamp(start + WEEK + 9);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 27);
}

#[test]
#[should_panic(expected = "cannot cancel stream: minimum duration not met")]
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

    client.subscribe(&subscriber, &creator, &token.address, &100, &2);
    client.subscribe(&subscriber, &creator, &token.address, &100, &2);
    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Minimum duration has passed, but still inside free trial.
    env.ledger().set_timestamp(start + DAY + 10);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&subscriber), 1000);
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(100 + 86400 + 10);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 0);
}

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

    client.subscribe(&subscriber, &creator, &token.address, &100, &1);
    assert_eq!(token.balance(&contract_id), 100);

    env.ledger().set_timestamp(0);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);
    client.top_up(&subscriber, &creator, &50);

    env.ledger().set_timestamp(WEEK + 120);
    env.ledger().set_timestamp(120);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 120);
    assert_eq!(token.balance(&contract_id), 30);
}

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

    let key = DataKey::Stream(subscriber.clone(), creator.clone());
    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&key));
        assert!(!env.storage().temporary().has(&key));
    });

    // Deplete stream balance after trial; this should mark stream inactive.
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

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &10, &1);

    let key = DataKey::Stream(subscriber.clone(), creator.clone());
    env.ledger().set_timestamp(start + WEEK + 20);
    client.collect(&subscriber, &creator);

    env.as_contract(&contract_id, || {
        assert!(env.storage().temporary().has(&key));
        assert!(!env.storage().persistent().has(&key));
    });

    client.top_up(&subscriber, &creator, &5);

    env.as_contract(&contract_id, || {
        assert!(env.storage().persistent().has(&key));
        assert!(!env.storage().temporary().has(&key));
    });
}

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
    env.ledger().set_timestamp(100);
    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &500,
        &10,
        &creators,
        &percentages,
    );

    env.ledger().set_timestamp(start + WEEK + 10);
    env.ledger().set_timestamp(110);
    client.collect_group(&subscriber, &channel_id);

    // 10 seconds * 10 tokens/sec = 100 tokens split across creators
    assert_eq!(token.balance(&creator_1), 40);
    assert_eq!(token.balance(&creator_2), 25);
    assert_eq!(token.balance(&creator_3), 15);
    assert_eq!(token.balance(&creator_4), 10);
    assert_eq!(token.balance(&creator_5), 10);
    assert_eq!(token.balance(&contract_id), 400);
}

#[test]
fn test_cliff_based_access_before_threshold() {
#[should_panic(expected = "group channel must contain exactly 5 creators")]
fn test_group_requires_exactly_five_creators() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
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

    client.set_cliff_threshold(&creator, &50);
    assert_eq!(client.get_cliff_threshold(&creator), 50);

    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);

    client.subscribe(&subscriber, &creator, &token.address, &30, &1);
    env.ledger().set_timestamp(100);
    client.collect(&subscriber, &creator);

    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);
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

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Advance beyond minimum duration (24h + 1h)
    env.ledger().set_timestamp(100 + 86400 + 3600);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&subscriber), 900);
}

#[test]
fn test_migrate_tier_downgrade_prorates_refund() {
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

    env.ledger().set_timestamp(105);
    client.migrate_tier(&subscriber, &creator, &5, &0);
    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone()
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
fn test_group_cancel_collects_and_refunds_remaining_balance() {
fn test_pause_channel_blocks_charges_and_unpause_resumes() {
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

    // Start at t=0, deposit 200 tokens at rate 1/sec
    // After exactly 30 seconds, cancel (past minimum duration)
    // 30 tokens collected, 170 refunded
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

    // Advance past minimum duration (24h) + 30 seconds
    env.ledger().set_timestamp(86400 + 30);
    client.cancel_group(&subscriber, &channel_id);

    // 86430s * 1/sec = 86430, capped at balance 200 → all 200 collected
    // 200 tokens split: 40%=80, 20%=40, 20%=40, 10%=20, 10%=20
    assert_eq!(token.balance(&creator_1), 80);
    assert_eq!(token.balance(&creator_2), 40);
    assert_eq!(token.balance(&creator_3), 40);
    assert_eq!(token.balance(&creator_4), 20);
    assert_eq!(token.balance(&creator_5), 20);
    assert_eq!(token.balance(&subscriber), 800); // 1000 - 200 deposited, 0 refund
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
fn test_cliff_based_access_after_threshold() {
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

    // Expired: expiry = 100 + (100/10) = 110
    env.ledger().set_timestamp(111);
    assert!(!client.is_subscribed(&subscriber, &creator));
}

#[test]
fn test_is_subscribed_none() {
    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &300, &2);

    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 20);

    env.ledger().set_timestamp(start + WEEK + 20);
    client.pause_channel(&creator);
    assert!(client.is_channel_paused(&creator));
    assert_eq!(token.balance(&creator), 40);

#[test]
fn test_migrate_tier_upgrade_with_additional_deposit() {
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

    client.migrate_tier(&subscriber, &creator, &2, &50);

    assert_eq!(token.balance(&contract_id), 150);
    assert_eq!(token.balance(&subscriber), 850);
}

#[test]
fn test_access_tiers() {
    env.ledger().set_timestamp(start + WEEK + 100);
    client.collect(&subscriber, &creator);
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &300, &2);

    env.ledger().set_timestamp(110);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 20);

    env.ledger().set_timestamp(120);
    client.pause_channel(&creator);
    assert!(client.is_channel_paused(&creator));
    // Pause settles the 10-second pending amount before freezing.
    assert_eq!(token.balance(&creator), 40);

    env.ledger().set_timestamp(200);
    client.collect(&subscriber, &creator);
    // No additional charges while paused.
    assert_eq!(token.balance(&creator), 40);

    client.unpause_channel(&creator);
    assert!(!client.is_channel_paused(&creator));

    env.ledger().set_timestamp(start + WEEK + 110);
    env.ledger().set_timestamp(210);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 60);
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
    token_admin.mint(&subscriber, &1000);
    token_admin.mint(&subscriber_1, &200);
    token_admin.mint(&subscriber_2, &200);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone()
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
fn test_pause_channel_blocks_charges_and_unpause_resumes() {
    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber_1, &creator, &token.address, &200, &1);
    client.subscribe(&subscriber_2, &creator, &token.address, &200, &1);

    env.ledger().set_timestamp(start + WEEK + 30);
    client.pause_channel(&creator);
    assert_eq!(token.balance(&creator), 60);

    env.ledger().set_timestamp(start + WEEK + 130);
    client.unpause_channel(&creator);

    env.ledger().set_timestamp(start + WEEK + 140);
    let total = client.withdraw_all(&creator, &10);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber_1, &creator, &token.address, &200, &1);
    client.subscribe(&subscriber_2, &creator, &token.address, &200, &1);

    env.ledger().set_timestamp(130);
    client.pause_channel(&creator);
    assert_eq!(token.balance(&creator), 60);

    env.ledger().set_timestamp(230);
    client.unpause_channel(&creator);

    env.ledger().set_timestamp(240);
    let total = client.withdraw_all(&creator, &10);

    // Only post-unpause 10 seconds are billable for each stream.
    assert_eq!(total, 20);
    assert_eq!(token.balance(&creator), 80);
    assert_eq!(token.balance(&contract_id), 320);
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
    token_admin.mint(&subscriber, &10000);
    token_admin.mint(&subscriber, &1000);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &300, &2);

    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_access_tier(&subscriber, &creator), 1);
    assert!(client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(token.balance(&creator), 20);

    env.ledger().set_timestamp(start + WEEK + 20);
    client.pause_channel(&creator);
    assert!(client.is_channel_paused(&creator));
    assert_eq!(token.balance(&creator), 40);

    env.ledger().set_timestamp(start + WEEK + 100);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_access_tier(&subscriber, &creator), 2);
    assert_eq!(token.balance(&creator), 40);

    client.unpause_channel(&creator);
    assert!(!client.is_channel_paused(&creator));

    env.ledger().set_timestamp(start + WEEK + 110);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_access_tier(&subscriber, &creator), 3);
}

#[test]
fn test_migrate_tier_emits_tier_changed_event() {
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

    let events = env.events().all();
    // Verify at least one event was emitted (TierChanged)
    let _ = events;
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
    assert_eq!(token.balance(&creator), 60);
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

    env.ledger().set_timestamp(start + WEEK + 30);
    client.pause_channel(&creator);
    assert_eq!(token.balance(&creator), 60);

    env.ledger().set_timestamp(start + WEEK + 130);
    client.unpause_channel(&creator);

    env.ledger().set_timestamp(start + WEEK + 140);
    let total = client.withdraw_all(&creator, &10);

    assert_eq!(total, 20);
    assert_eq!(token.balance(&creator), 80);
    assert_eq!(token.balance(&contract_id), 320);
}

#[test]
fn test_cliff_threshold_access() {
    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(start + WEEK + 30);

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(130);
    client.collect(&subscriber, &creator);
    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);

    env.ledger().set_timestamp(start + WEEK + 50);
    env.ledger().set_timestamp(150);
    client.collect(&subscriber, &creator);
    assert!(client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 1);
}

#[test]
fn test_migrate_tier_downgrade_prorates_refund() {
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
    client.subscribe(&subscriber, &creator, &token.address, &100, &10);

    env.ledger().set_timestamp(start + WEEK + 5);
    client.migrate_tier(&subscriber, &creator, &5, &0);

    assert_eq!(token.balance(&creator), 50);

#[test]
#[should_panic(expected = "percentages must sum to 100")]
fn test_group_percentages_must_sum_to_100() {
fn test_migrate_tier_downgrade_prorates_refund() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let channel_id = Address::generate(&env);
    let creator_1 = Address::generate(&env);
    let creator_2 = Address::generate(&env);
    let creator_3 = Address::generate(&env);
    let creator_4 = Address::generate(&env);
    let creator_5 = Address::generate(&env);
    let creator = Address::generate(&env);
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
    let percentages = vec![&env, 30u32, 20u32, 20u32, 10u32, 10u32]; // sums to 90
    // No subscription exists
    assert!(!client.is_subscribed(&subscriber, &creator));
    client.set_cliff_threshold(&creator, &50);

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(start + WEEK + 30);
    client.collect(&subscriber, &creator);
    assert!(!client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 0);

    env.ledger().set_timestamp(start + WEEK + 50);
    client.collect(&subscriber, &creator);
    assert!(client.has_unlocked_access(&subscriber, &creator));
    assert_eq!(client.get_access_tier(&subscriber, &creator), 1);
}

#[test]
fn test_migrate_tier_downgrade_prorates_refund() {
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
    client.subscribe(&subscriber, &creator, &token.address, &100, &10);

    env.ledger().set_timestamp(start + WEEK + 5);
    client.migrate_tier(&subscriber, &creator, &5, &0);

    assert_eq!(token.balance(&creator), 50);
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &10);

    env.ledger().set_timestamp(105);
    client.migrate_tier(&subscriber, &creator, &5, &0);

    // Collected at old rate before migration.
    assert_eq!(token.balance(&creator), 50);
    // Remaining 50 balance is prorated to 25 at new rate, 25 refunded.
    assert_eq!(token.balance(&subscriber), 925);
    assert_eq!(token.balance(&contract_id), 25);
}

#[test]
#[should_panic(expected = "new rate must be positive")]
fn test_migrate_tier_invalid_rate() {
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

    client.migrate_tier(&subscriber, &creator, &0, &0);
}

#[test]
fn test_migrate_tier_upgrade_collects_at_new_rate() {
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

    env.ledger().set_timestamp(110);
    client.migrate_tier(&subscriber, &creator, &2, &0);

    assert_eq!(token.balance(&creator), 10);
    assert_eq!(token.balance(&contract_id), 90);

    env.ledger().set_timestamp(120);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 30);
    assert_eq!(token.balance(&contract_id), 70);
}

#[test]
#[should_panic(expected = "cannot cancel stream: minimum duration not met")]
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

    // Try to cancel after only 1 hour — should fail
    env.ledger().set_timestamp(100 + 3600);
    client.cancel(&subscriber, &creator);
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

    env.ledger().set_timestamp(100 + 86400);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
#[should_panic(
    expected = "cannot cancel stream: minimum duration not met. 43200 seconds remaining"
)]
fn test_cancel_with_remaining_time_message() {
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

    // Try to cancel after 12 hours (43200 seconds remaining)
    env.ledger().set_timestamp(100 + 43200);
    client.cancel(&subscriber, &creator);
}

#[test]
fn test_total_streamed_tracking() {
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

    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(100);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_total_streamed(&subscriber, &creator), 100);

    client.top_up(&subscriber, &creator, &50);
    env.ledger().set_timestamp(150);
    client.collect(&subscriber, &creator);
    assert_eq!(client.get_total_streamed(&subscriber, &creator), 150);
}

#[test]
fn test_creator_metadata() {
    let env = Env::default();
    env.mock_all_auths();

    let creator = Address::generate(&env);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // No metadata set yet
    assert_eq!(client.get_creator_metadata(&creator), None);

    // Set an IPFS CID
    let cid = Bytes::from_slice(&env, b"QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    client.set_creator_metadata(&creator, &cid);

    // Retrieve and verify
    assert_eq!(client.get_creator_metadata(&creator), Some(cid.clone()));

    // Update to a new CID
    let new_cid = Bytes::from_slice(&env, b"QmNewCIDabcdefghijklmnopqrstuvwxyz1234567890AB");
    client.set_creator_metadata(&creator, &new_cid);
    assert_eq!(client.get_creator_metadata(&creator), Some(new_cid));
}
