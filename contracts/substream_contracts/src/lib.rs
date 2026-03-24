#![no_std]
use soroban_sdk::{ contract, contractimpl, contracttype, Address, Env };
use soroban_sdk::token::Client as TokenClient;

// Minimum flow duration: 24 hours in seconds (24 * 60 * 60 = 86400)
const MINIMUM_FLOW_DURATION: u64 = 86400;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address), // (subscriber, creator)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    pub token: Address,
    pub rate_per_second: i128,
    pub balance: i128,
    pub last_collected: u64,
    pub start_time: u64,
}

#[contract]
pub struct SubStreamContract;

#[contractimpl]
impl SubStreamContract {
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        creator: Address,
        token: Address,
        amount: i128,
        rate_per_second: i128
    ) {
        subscriber.require_auth();

        if amount <= 0 || rate_per_second <= 0 {
            panic!("amount and rate must be positive");
        }

        let key = DataKey::Stream(subscriber.clone(), creator.clone());
        if env.storage().persistent().has(&key) {
            panic!("stream already exists");
        }

        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&subscriber, &env.current_contract_address(), &amount);

        let current_time = env.ledger().timestamp();
        let stream = Stream {
            token,
            rate_per_second,
            balance: amount,
            last_collected: current_time,
            start_time: current_time,
        };

        env.storage().persistent().set(&key, &stream);
    }

    pub fn collect(env: Env, subscriber: Address, creator: Address) {
        let key = DataKey::Stream(subscriber.clone(), creator.clone());
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
        let current_time = env.ledger().timestamp();

        if current_time <= stream.last_collected {
            return;
        }

        let time_elapsed = (current_time - stream.last_collected) as i128;
        let mut amount_to_collect = time_elapsed * stream.rate_per_second;

        if amount_to_collect > stream.balance {
            amount_to_collect = stream.balance;
        }

        if amount_to_collect > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            token_client.transfer(&env.current_contract_address(), &creator, &amount_to_collect);

            stream.balance -= amount_to_collect;
            stream.last_collected = current_time;

            env.storage().persistent().set(&key, &stream);
        }
    }

    pub fn cancel(env: Env, subscriber: Address, creator: Address) {
        subscriber.require_auth();

        let key = DataKey::Stream(subscriber.clone(), creator.clone());
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        // Get stream to check minimum duration
        let stream: Stream = env.storage().persistent().get(&key).unwrap();
        let current_time = env.ledger().timestamp();

        // Check if minimum flow duration has been met
        if current_time < stream.start_time + MINIMUM_FLOW_DURATION {
            let remaining_time = stream.start_time + MINIMUM_FLOW_DURATION - current_time;
            panic!("cannot cancel stream: minimum duration not met. {} seconds remaining", remaining_time);
        }

        // First collect any pending amount
        Self::collect(env.clone(), subscriber.clone(), creator.clone());

        // Get updated stream
        let stream: Stream = env.storage().persistent().get(&key).unwrap();

        // Refund remaining balance to subscriber
        if stream.balance > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            token_client.transfer(&env.current_contract_address(), &subscriber, &stream.balance);
        }

        // Remove the stream from storage
        env.storage().persistent().remove(&key);
    }

    pub fn top_up(env: Env, subscriber: Address, creator: Address, amount: i128) {
        subscriber.require_auth();
        if amount <= 0 {
            panic!("amount must be positive");
        }

        let key = DataKey::Stream(subscriber.clone(), creator.clone());
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
        let token_client = TokenClient::new(&env, &stream.token);
        token_client.transfer(&subscriber, &env.current_contract_address(), &amount);

        stream.balance += amount;
        env.storage().persistent().set(&key, &stream);
    }
}

mod test;
