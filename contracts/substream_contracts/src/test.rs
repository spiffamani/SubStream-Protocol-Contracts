#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

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
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

    // No subscription exists
    assert!(!client.is_subscribed(&subscriber, &creator));
}
