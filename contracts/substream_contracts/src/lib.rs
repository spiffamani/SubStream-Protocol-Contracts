#![no_std]
use soroban_sdk::contractevent;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, Vec};

// Minimum flow duration: 24 hours in seconds (24 * 60 * 60 = 86400)
const MINIMUM_FLOW_DURATION: u64 = 86400;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address),        // (subscriber, creator)
    TotalStreamed(Address, Address), // (subscriber, creator)
    CliffThreshold(Address),         // creator -> threshold amount for access
    CreatorSubscribers(Address),     // creator -> Vec<subscriber>
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    pub token: Address,
    pub rate_per_second: i128,
    pub balance: i128,
    pub last_collected: u64,
    pub start_time: u64,
    pub creators: Vec<Address>,
    pub percentages: Vec<u32>,
}

#[contractevent]
pub struct TierChanged {
    #[topic]
    pub subscriber: Address,
    #[topic]
    pub creator: Address,
    pub old_rate: i128,
    pub new_rate: i128,
}

#[contract]
pub struct SubStreamContract;

fn stream_key(subscriber: &Address, stream_id: &Address) -> DataKey {
    DataKey::Stream(subscriber.clone(), stream_id.clone())
}

fn validate_distribution(creators: &Vec<Address>, percentages: &Vec<u32>, expected: u32) {
    let creators_len = creators.len();
    if creators_len != expected {
        panic!("invalid creator count");
    }
    if percentages.len() != creators_len {
        panic!("creators and percentages length mismatch");
    }
    let mut total: u32 = 0;
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

#[contractimpl]
impl SubStreamContract {
    // Single-creator subscribe for backwards compatibility
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        creator: Address,
        token: Address,
        amount: i128,
        rate_per_second: i128,
    ) {
        let creators = vec![&env, creator.clone()];
        let percentages = vec![&env, 100u32];
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

    pub fn collect(env: Env, subscriber: Address, stream_id: Address) {
        collect_internal(&env, &subscriber, &stream_id);
    }

    pub fn cancel(env: Env, subscriber: Address, stream_id: Address) {
        cancel_internal(&env, &subscriber, &stream_id);
    }

    pub fn top_up(env: Env, subscriber: Address, stream_id: Address, amount: i128) {
        top_up_internal(&env, &subscriber, &stream_id, amount);
    }

    /// View: returns true only if the user has active funds remaining (not expired)
    pub fn is_subscribed(env: Env, subscriber: Address, creator: Address) -> bool {
        let key = stream_key(&subscriber, &creator);
        if !env.storage().persistent().has(&key) {
            return false;
        }
        let stream: Stream = env.storage().persistent().get(&key).unwrap();
        if stream.rate_per_second <= 0 || stream.balance <= 0 {
            return false;
        }
        let secs = (stream.balance / stream.rate_per_second) as i128;
        if secs <= 0 {
            return false;
        }
        let expiry = stream.start_time.saturating_add(secs as u64);
        let current = env.ledger().timestamp();
        current < expiry
    }

    // Minimal group wrappers
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

    pub fn set_cliff_threshold(env: Env, creator: Address, threshold: i128) {
        creator.require_auth();
        if threshold < 0 {
            panic!("threshold must be non-negative");
        }
        let key = DataKey::CliffThreshold(creator.clone());
        env.storage().persistent().set(&key, &threshold);
    }

    pub fn get_cliff_threshold(env: Env, creator: Address) -> i128 {
        let key = DataKey::CliffThreshold(creator.clone());
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn get_total_streamed(env: Env, subscriber: Address, creator: Address) -> i128 {
        let key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        env.storage().persistent().get(&key).unwrap_or(0)
    }

    pub fn has_unlocked_access(env: Env, subscriber: Address, creator: Address) -> bool {
        let threshold_key = DataKey::CliffThreshold(creator.clone());
        let threshold: i128 = env.storage().persistent().get(&threshold_key).unwrap_or(0);
        if threshold == 0 {
            return true;
        }
        let streamed_key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        let total_streamed: i128 = env.storage().persistent().get(&streamed_key).unwrap_or(0);
        total_streamed >= threshold
    }

    pub fn get_access_tier(env: Env, subscriber: Address, creator: Address) -> u32 {
        let threshold_key = DataKey::CliffThreshold(creator.clone());
        let threshold: i128 = env.storage().persistent().get(&threshold_key).unwrap_or(0);
        if threshold == 0 {
            return 2;
        }
        let streamed_key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        let total_streamed: i128 = env.storage().persistent().get(&streamed_key).unwrap_or(0);
        if total_streamed >= 500 {
            3
        } else if total_streamed >= 200 {
            2
        } else if total_streamed >= 50 {
            1
        } else {
            0
        }
    }

    fn update_total_streamed(env: &Env, subscriber: &Address, creator: &Address, amount: i128) {
        let key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        let current_total: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        let new_total = current_total + amount;
        env.storage().persistent().set(&key, &new_total);
    }
}

// Internal implementations
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
    let now = env.ledger().timestamp();
    let stream = Stream {
        token: token.clone(),
        rate_per_second,
        balance: amount,
        last_collected: now,
        start_time: now,
        creators,
        percentages,
    };
    env.storage().persistent().set(&key, &stream);
    // track subscriber in creator list for single-creator case
    if let DataKey::Stream(_, creator_addr) = &key {
        let creator_key = DataKey::CreatorSubscribers(creator_addr.clone());
        let mut subs: Vec<Address> = env
            .storage()
            .persistent()
            .get(&creator_key)
            .unwrap_or(vec![env]);
        subs.push_back(subscriber.clone());
        env.storage().persistent().set(&creator_key, &subs);
    }
}

fn collect_internal(env: &Env, subscriber: &Address, stream_id: &Address) {
    let key = stream_key(subscriber, stream_id);
    if !env.storage().persistent().has(&key) {
        panic!("stream not found");
    }
    let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
    let current_time = env.ledger().timestamp();
    if current_time <= stream.last_collected || stream.balance == 0 {
        return;
    }
    let time_elapsed = (current_time - stream.last_collected) as i128;
    let mut amount_to_collect = time_elapsed
        .checked_mul(stream.rate_per_second)
        .unwrap_or(0);
    if amount_to_collect > stream.balance {
        amount_to_collect = stream.balance;
    }
    if amount_to_collect <= 0 {
        return;
    }
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
    if let DataKey::Stream(_, creator_addr) = &key {
        SubStreamContract::update_total_streamed(env, subscriber, creator_addr, amount_to_collect);
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

mod test;
