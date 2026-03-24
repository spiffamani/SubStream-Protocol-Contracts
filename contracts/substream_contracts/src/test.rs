#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, vec, Address, Env,
};

const DAY: u64 = 24 * 60 * 60;
const WEEK: u64 = 7 * DAY;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &sac.address())
}

#[test]
fn test_subscribe_and_collect() {
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
    client.subscribe(&subscriber, &creator, &token.address, &100, &2);

    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 100);

    // Still inside trial: no charges.
    env.ledger().set_timestamp(start + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 0);

    // 10 paid seconds after trial.
    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 20);
    assert_eq!(token.balance(&contract_id), 80);

    // Additional 50 paid seconds, capped by remaining balance.
    env.ledger().set_timestamp(start + WEEK + 60);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(client.get_total_streamed(&subscriber, &creator), 100);
}

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

    let start = 100u64;
    env.ledger().set_timestamp(start);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Minimum duration has passed, but still inside free trial.
    env.ledger().set_timestamp(start + DAY + 10);
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 0);
    assert_eq!(token.balance(&subscriber), 1000);
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

    env.ledger().set_timestamp(0);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);
    client.top_up(&subscriber, &creator, &50);

    env.ledger().set_timestamp(WEEK + 120);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 120);
    assert_eq!(token.balance(&contract_id), 30);
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
        creator_5.clone()
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
    client.subscribe(&subscriber, &creator, &token.address, &300, &2);

    env.ledger().set_timestamp(start + WEEK + 10);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 20);

    env.ledger().set_timestamp(start + WEEK + 20);
    client.pause_channel(&creator);
    assert!(client.is_channel_paused(&creator));
    assert_eq!(token.balance(&creator), 40);

    env.ledger().set_timestamp(start + WEEK + 100);
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 40);

    client.unpause_channel(&creator);
    assert!(!client.is_channel_paused(&creator));

    env.ledger().set_timestamp(start + WEEK + 110);
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
    assert_eq!(token.balance(&subscriber), 925);
    assert_eq!(token.balance(&contract_id), 25);
}
