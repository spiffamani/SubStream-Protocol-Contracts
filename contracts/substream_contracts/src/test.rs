#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

const DAY: u64 = 24 * 60 * 60;
const WEEK: u64 = 7 * DAY;

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let sac = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &sac.address())
}

#[test]
fn test_grace_period_access_and_debt() {
    let env = Env::default();
    env.mock_all_auths();

    let subscriber = Address::generate(&env);
    let creator = Address::generate(&env);
    let admin = Address::generate(&env);

    let token = create_token_contract(&env, &admin);
    let token_admin = token::StellarAssetClient::new(&env, &token.address);
    token_admin.mint(&subscriber, &10000000); // Plenty of tokens for testing

    let contract_id = env.register(SubStreamContract, ());
    let client = SubStreamContractClient::new(&env, &contract_id);

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
