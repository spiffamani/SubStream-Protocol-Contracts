#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Address as _, Events as _, Ledger};
use soroban_sdk::{token, Address, Env, Event};
use soroban_sdk::{testutils::{Address as _, Ledger}, token, vec, Address, Env};

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

    // Initial timestamp
    env.ledger().set_timestamp(100);

    // Subscribe: 100 tokens, rate 2 per second
    client.subscribe(&subscriber, &creator, &token.address, &100, &2);

    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 100);

    // Advance 10 seconds
    env.ledger().set_timestamp(110);

    // Collect: 10 secs * 2 tokens/sec = 20 tokens
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&contract_id), 80);
    assert_eq!(token.balance(&creator), 20);

    // Advance 50 seconds (would be 100 tokens, but only 80 left in balance)
    env.ledger().set_timestamp(160);
    client.collect(&subscriber, &creator);

    assert_eq!(token.balance(&contract_id), 0);
    assert_eq!(token.balance(&creator), 100);
}

#[test]
fn test_cancel() {
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

    // Subscribe: 100 tokens, 1 token/sec
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    env.ledger().set_timestamp(120); // 20 seconds pass

    // Cancel should collect 20 for creator, refund 80 to subscriber
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 20);
    assert_eq!(token.balance(&subscriber), 980);
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
#[should_panic(expected = "amount and rate must be positive")]
fn test_subscribe_invalid_amounts() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    client.subscribe(&subscriber, &creator, &token, &0, &2);
}

#[test]
#[should_panic(expected = "stream already exists")]
fn test_subscribe_already_exists() {
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
    // Should panic here
    client.subscribe(&subscriber, &creator, &token.address, &100, &2);
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

    // Initial subscribe
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);
    assert_eq!(token.balance(&contract_id), 100);

    // Top up
    client.top_up(&subscriber, &creator, &50);
    assert_eq!(token.balance(&contract_id), 150);

    // Verify it still works with the new balance
    env.ledger().set_timestamp(120); // 120 seconds pass
    client.collect(&subscriber, &creator);
    assert_eq!(token.balance(&creator), 120);
    assert_eq!(token.balance(&contract_id), 30);
}


#[test]
fn test_withdraw_all() {
    let env = Env::default();
    env.mock_all_auths();

#[test]
fn test_migrate_tier_upgrade_collects_at_new_rate() {
fn test_group_subscribe_and_collect_split() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
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
fn test_migrate_tier_downgrade_prorates_refund() {
    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone()
    ];
    let percentages = vec![&env, 40u32, 25u32, 15u32, 10u32, 10u32];

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

    assert_eq!(token.balance(&subscriber), 500);
    assert_eq!(token.balance(&contract_id), 500);

    env.ledger().set_timestamp(110);
    client.collect_group(&subscriber, &channel_id);

    // 10 seconds * 10 tokens/sec = 100 tokens split across creators.
    assert_eq!(token.balance(&creator_1), 40);
    assert_eq!(token.balance(&creator_2), 25);
    assert_eq!(token.balance(&creator_3), 15);
    assert_eq!(token.balance(&creator_4), 10);
    assert_eq!(token.balance(&creator_5), 10);
    assert_eq!(token.balance(&contract_id), 400);
}

#[test]
fn test_group_cancel_collects_and_refunds_remaining_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
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

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &10);

    env.ledger().set_timestamp(105);
    client.migrate_tier(&subscriber, &creator, &5, &0);

    assert_eq!(token.balance(&creator), 50);
    assert_eq!(token.balance(&contract_id), 25);
    assert_eq!(token.balance(&subscriber), 925);
}

#[test]
fn test_migrate_tier_upgrade_with_additional_deposit() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);

    // Three subscribers, each deposits 100 tokens at rate 1/sec
    let sub1 = Address::generate(&env);
    let sub2 = Address::generate(&env);
    let sub3 = Address::generate(&env);
    token_admin.mint(&sub1, &100);
    token_admin.mint(&sub2, &100);
    token_admin.mint(&sub3, &100);
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
fn test_migrate_tier_emits_tier_changed_event() {
    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone()
    ];
    let percentages = vec![&env, 40u32, 20u32, 20u32, 10u32, 10u32];

    env.ledger().set_timestamp(100);
    client.subscribe_group(
        &subscriber,
        &channel_id,
        &token.address,
        &200,
        &2,
        &creators,
        &percentages,
    );

    env.ledger().set_timestamp(130); // 30s * 2 = 60 collected
    client.cancel_group(&subscriber, &channel_id);

    assert_eq!(token.balance(&creator_1), 24);
    assert_eq!(token.balance(&creator_2), 12);
    assert_eq!(token.balance(&creator_3), 12);
    assert_eq!(token.balance(&creator_4), 6);
    assert_eq!(token.balance(&creator_5), 6);
    assert_eq!(token.balance(&subscriber), 940); // 1000 - 200 + 140 refund
    assert_eq!(token.balance(&contract_id), 0);
}

#[test]
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

    env.ledger().set_timestamp(0);
    client.subscribe(&sub1, &creator, &token.address, &100, &1);
    client.subscribe(&sub2, &creator, &token.address, &100, &1);
    client.subscribe(&sub3, &creator, &token.address, &100, &1);

    // Advance 10 seconds — each stream owes 10 tokens = 30 total
    env.ledger().set_timestamp(10);

    let collected = client.withdraw_all(&creator, &10);

    assert_eq!(collected, 30);
    assert_eq!(token.balance(&creator), 30);
    assert_eq!(token.balance(&contract_id), 270); // 300 deposited - 30 collected
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    client.migrate_tier(&subscriber, &creator, &3, &0);

    let expected = TierChanged {
        subscriber: subscriber.clone(),
        creator: creator.clone(),
        old_rate: 1,
        new_rate: 3,
    }
    .to_xdr(&env, &contract_id);

    assert_eq!(
        env.events().all().filter_by_contract(&contract_id),
        [expected],
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
#[should_panic(expected = "new rate must be positive")]
fn test_migrate_tier_invalid_rate() {
#[should_panic(expected = "percentages must sum to 100")]
fn test_group_percentages_must_sum_to_100() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
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

    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    client.migrate_tier(&subscriber, &creator, &0, &0);
    let creators = vec![
        &env,
        creator_1.clone(),
        creator_2.clone(),
        creator_3.clone(),
        creator_4.clone(),
        creator_5.clone()
    ];
    let percentages = vec![&env, 30u32, 20u32, 20u32, 10u32, 10u32]; // 90

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
