#![cfg(test)]

use super::*;
use soroban_sdk::{ testutils::{ Address as _, Ledger }, token, Address, Env };

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    token::Client::new(env, &env.register_stellar_asset_contract(admin.clone()))
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

    env.ledger().set_timestamp(100 + 86400 + 120); // 24h + 120 seconds pass (respect minimum duration)

    // Cancel should collect 100 tokens (all balance depleted) for creator, refund 0 to subscriber
    client.cancel(&subscriber, &creator);

    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&subscriber), 900);
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

    // Subscribe at timestamp 100
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Try to cancel after only 1 hour (3600 seconds) - should fail
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

    // Subscribe at timestamp 100
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Advance time beyond minimum duration (24 hours + 1 hour)
    env.ledger().set_timestamp(100 + 86400 + 3600);

    // Cancel should work now
    client.cancel(&subscriber, &creator);

    // Verify final state: creator gets all 100 tokens (balance depleted), subscriber gets no refund
    assert_eq!(token.balance(&creator), 100);
    assert_eq!(token.balance(&subscriber), 900);
    assert_eq!(token.balance(&contract_id), 0);
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

    // Subscribe at timestamp 100
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Cancel exactly at minimum duration (24 hours later)
    env.ledger().set_timestamp(100 + 86400);

    // Cancel should work exactly at minimum duration
    client.cancel(&subscriber, &creator);

    // Verify final state: creator gets all 100 tokens (balance depleted), subscriber gets no refund
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

    // Subscribe at timestamp 100
    env.ledger().set_timestamp(100);
    client.subscribe(&subscriber, &creator, &token.address, &100, &1);

    // Try to cancel after 12 hours (43200 seconds remaining)
    env.ledger().set_timestamp(100 + 43200);
    client.cancel(&subscriber, &creator);
}
