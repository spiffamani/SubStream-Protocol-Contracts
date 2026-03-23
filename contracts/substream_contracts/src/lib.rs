#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};
use soroban_sdk::token::Client as TokenClient;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address), // (subscriber, stream_id)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    pub token: Address,
    pub rate_per_second: i128,
    pub balance: i128,
    pub last_collected: u64,
    pub creators: Vec<Address>,
    pub percentages: Vec<u32>,
}

#[contract]
pub struct SubStreamContract;

fn stream_key(subscriber: &Address, stream_id: &Address) -> DataKey {
    DataKey::Stream(subscriber.clone(), stream_id.clone())
}

fn validate_distribution(
    creators: &Vec<Address>,
    percentages: &Vec<u32>,
    expected_creator_count: u32,
) {
    if creators.len() != expected_creator_count {
        if expected_creator_count == 5 {
            panic!("group channel must contain exactly 5 creators");
        }
        panic!("invalid creator count");
    }

    if percentages.len() != creators.len() {
        panic!("creators and percentages length mismatch");
    }

    let mut total: u32 = 0;
    let creators_len = creators.len();
    for i in 0..creators_len {
        let percentage = percentages.get(i).unwrap();
        if percentage == 0 {
            panic!("percentages must be positive");
        }
        total += percentage;

        let creator_i = creators.get(i).unwrap();
        for j in (i + 1)..creators_len {
            if creator_i == creators.get(j).unwrap() {
                panic!("creators must be unique");
            }
        }
    }

    if total != 100 {
        panic!("percentages must sum to 100");
    }
}

fn subscribe_internal(
    env: &Env,
    subscriber: &Address,
    stream_id: &Address,
    token: &Address,
    amount: i128,
    rate_per_second: i128,
    creators: Vec<Address>,
    percentages: Vec<u32>,
) {
    subscriber.require_auth();

    if amount <= 0 || rate_per_second <= 0 {
        panic!("amount and rate must be positive");
    }

    let key = stream_key(subscriber, stream_id);
    if env.storage().persistent().has(&key) {
        panic!("stream already exists");
    }

    let token_client = TokenClient::new(env, token);
    token_client.transfer(subscriber, &env.current_contract_address(), &amount);

    let stream = Stream {
        token: token.clone(),
        rate_per_second,
        balance: amount,
        last_collected: env.ledger().timestamp(),
        creators,
        percentages,
    };

    env.storage().persistent().set(&key, &stream);
}

fn collect_internal(env: &Env, subscriber: &Address, stream_id: &Address) {
    let key = stream_key(subscriber, stream_id);
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
        let token_client = TokenClient::new(env, &stream.token);
        let mut remaining = amount_to_collect;
        let creators_len = stream.creators.len();

        for i in 0..creators_len {
            let creator = stream.creators.get(i).unwrap();
            let payout = if (i + 1) == creators_len {
                remaining
            } else {
                let percentage = stream.percentages.get(i).unwrap() as i128;
                let amount = (amount_to_collect * percentage) / 100;
                remaining -= amount;
                amount
            };

            if payout > 0 {
                token_client.transfer(&env.current_contract_address(), &creator, &payout);
            }
        }

        stream.balance -= amount_to_collect;
        stream.last_collected = current_time;
        env.storage().persistent().set(&key, &stream);
    }
}

fn cancel_internal(env: &Env, subscriber: &Address, stream_id: &Address) {
    subscriber.require_auth();

    let key = stream_key(subscriber, stream_id);
    if !env.storage().persistent().has(&key) {
        panic!("stream not found");
    }

    collect_internal(env, subscriber, stream_id);

    let stream: Stream = env.storage().persistent().get(&key).unwrap();
    if stream.balance > 0 {
        let token_client = TokenClient::new(env, &stream.token);
        token_client.transfer(&env.current_contract_address(), subscriber, &stream.balance);
    }

    env.storage().persistent().remove(&key);
}

fn top_up_internal(env: &Env, subscriber: &Address, stream_id: &Address, amount: i128) {
    subscriber.require_auth();
    if amount <= 0 {
        panic!("amount must be positive");
    }

    let key = stream_key(subscriber, stream_id);
    if !env.storage().persistent().has(&key) {
        panic!("stream not found");
    }

    let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
    let token_client = TokenClient::new(env, &stream.token);
    token_client.transfer(subscriber, &env.current_contract_address(), &amount);

    stream.balance += amount;
    env.storage().persistent().set(&key, &stream);
}

#[contractimpl]
impl SubStreamContract {
    // Existing single-creator interface preserved for backwards compatibility.
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        creator: Address,
        token: Address,
        amount: i128,
        rate_per_second: i128,
    ) {
        let creators = soroban_sdk::vec![&env, creator.clone()];
        let percentages = soroban_sdk::vec![&env, 100u32];
        validate_distribution(&creators, &percentages, 1);
        subscribe_internal(
            &env,
            &subscriber,
            &creator,
            &token,
            amount,
            rate_per_second,
            creators,
            percentages,
        );
    }

    pub fn collect(env: Env, subscriber: Address, creator: Address) {
        collect_internal(&env, &subscriber, &creator);
    }

    pub fn cancel(env: Env, subscriber: Address, creator: Address) {
        cancel_internal(&env, &subscriber, &creator);
    }

    pub fn top_up(env: Env, subscriber: Address, creator: Address, amount: i128) {
        top_up_internal(&env, &subscriber, &creator, amount);
    }

    // Group channel interface: one stream split across exactly 5 creators.
    pub fn subscribe_group(
        env: Env,
        subscriber: Address,
        channel_id: Address,
        token: Address,
        amount: i128,
        rate_per_second: i128,
        creators: Vec<Address>,
        percentages: Vec<u32>,
    ) {
        validate_distribution(&creators, &percentages, 5);
        subscribe_internal(
            &env,
            &subscriber,
            &channel_id,
            &token,
            amount,
            rate_per_second,
            creators,
            percentages,
        );
    }

    pub fn collect_group(env: Env, subscriber: Address, channel_id: Address) {
        collect_internal(&env, &subscriber, &channel_id);
    }

    pub fn cancel_group(env: Env, subscriber: Address, channel_id: Address) {
        cancel_internal(&env, &subscriber, &channel_id);
    }

    pub fn top_up_group(env: Env, subscriber: Address, channel_id: Address, amount: i128) {
        top_up_internal(&env, &subscriber, &channel_id, amount);
    }
}

mod test;
