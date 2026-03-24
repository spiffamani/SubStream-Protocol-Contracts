#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, vec, Address, Env, Vec};
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Vec};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address),      // (subscriber, creator)
    CreatorSubscribers(Address),   // creator -> Vec<subscriber>
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

#[contractimpl]
impl SubStreamContract {
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        creator: Address,
        token: Address,
        amount: i128,
        rate_per_second: i128,
    ) {
        subscriber.require_auth();

        if amount <= 0 || rate_per_second <= 0 {
            panic!("amount and rate must be positive");
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

        env.storage().persistent().set(&key, &stream);

        // Track subscriber under this creator for withdraw_all
        let creator_key = DataKey::CreatorSubscribers(creator.clone());
        let mut subs: Vec<Address> = env.storage().persistent()
            .get(&creator_key)
            .unwrap_or(vec![&env]);
        subs.push_back(subscriber);
        env.storage().persistent().set(&creator_key, &subs);
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

        let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
        let current_time = env.ledger().timestamp();

        if current_time <= stream.last_collected {
            return;
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

        if amount_to_collect > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            token_client.transfer(
                &env.current_contract_address(),
                &creator,
                &amount_to_collect,
            );

            stream.balance -= amount_to_collect;
            stream.last_collected = current_time;

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

        // Get updated stream
        let stream: Stream = env.storage().persistent().get(&key).unwrap();

        // Refund remaining balance to subscriber
        if stream.balance > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            token_client.transfer(
                &env.current_contract_address(),
                &subscriber,
                &stream.balance,
            );
            if payout > 0 {
                token_client.transfer(&env.current_contract_address(), &creator, &payout);
            }
        }

        // Remove the stream from storage
        env.storage().persistent().remove(&key);

        // Remove subscriber from creator's subscriber list
        let creator_key = DataKey::CreatorSubscribers(creator.clone());
        if let Some(subs) = env.storage().persistent().get::<DataKey, Vec<Address>>(&creator_key) {
            let mut updated: Vec<Address> = vec![&env];
            for s in subs.iter() {
                if s != subscriber {
                    updated.push_back(s);
                }
            }
            env.storage().persistent().set(&creator_key, &updated);
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

    /// Change the stream rate (tier) in one transaction without removing the stream.
    /// Pending payouts at the previous rate are settled via `collect` first.
    /// On downgrade (lower rate), excess buffer is prorated and refunded to the subscriber.
    /// On upgrade, `additional_deposit` can add tokens in the same transaction (use 0 if none).
    pub fn migrate_tier(
        env: Env,
        subscriber: Address,
        creator: Address,
        new_rate_per_second: i128,
        additional_deposit: i128,
    ) {
        subscriber.require_auth();

        if new_rate_per_second <= 0 {
            panic!("new rate must be positive");
        }
        if additional_deposit < 0 {
            panic!("additional deposit must be non-negative");
        }

        let key = DataKey::Stream(subscriber.clone(), creator.clone());
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        let stream_before: Stream = env.storage().persistent().get(&key).unwrap();
        let old_rate = stream_before.rate_per_second;

        Self::collect(env.clone(), subscriber.clone(), creator.clone());

        let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
        let mut balance = stream.balance;

        if new_rate_per_second < old_rate && balance > 0 {
            let tokens_to_keep = balance
                .checked_mul(new_rate_per_second)
                .expect("overflow")
                .checked_div(old_rate)
                .expect("old rate must be positive");
            let refund = balance.saturating_sub(tokens_to_keep);
            if refund > 0 {
                let token_client = TokenClient::new(&env, &stream.token);
                token_client.transfer(&env.current_contract_address(), &subscriber, &refund);
                balance = tokens_to_keep;
            }
        }

        stream.rate_per_second = new_rate_per_second;
        stream.balance = balance;

        if additional_deposit > 0 {
            let token_client = TokenClient::new(&env, &stream.token);
            token_client.transfer(
                &subscriber,
                &env.current_contract_address(),
                &additional_deposit,
            );
            stream.balance = stream
                .balance
                .checked_add(additional_deposit)
                .expect("overflow");
        }

        env.storage().persistent().set(&key, &stream);

        if old_rate != new_rate_per_second {
            TierChanged {
                subscriber: subscriber.clone(),
                creator: creator.clone(),
                old_rate,
                new_rate: new_rate_per_second,
            }
            .publish(&env);
        }
    }
    /// Collect from all active streams for a creator in a single call.
    /// `max_count` caps the batch size to avoid hitting ledger instruction limits.
    /// Call repeatedly with the same max_count to drain remaining subscribers.
    /// Returns the total amount collected across all processed streams.
    pub fn withdraw_all(env: Env, creator: Address, max_count: u32) -> i128 {
        let creator_key = DataKey::CreatorSubscribers(creator.clone());
        let subs: Vec<Address> = env.storage().persistent()
            .get(&creator_key)
            .unwrap_or(vec![&env]);

        let mut total_collected: i128 = 0;
        let limit = max_count.min(subs.len()) as usize;

        for i in 0..limit {
            let subscriber = subs.get(i as u32).unwrap();
            let stream_key = DataKey::Stream(subscriber.clone(), creator.clone());

            if !env.storage().persistent().has(&stream_key) {
                continue;
            }

            let mut stream: Stream = env.storage().persistent().get(&stream_key).unwrap();
            let current_time = env.ledger().timestamp();

            if current_time <= stream.last_collected || stream.balance == 0 {
                continue;
            }

            let time_elapsed = (current_time - stream.last_collected) as i128;
            let mut claimable = time_elapsed * stream.rate_per_second;
            if claimable > stream.balance {
                claimable = stream.balance;
            }

            if claimable > 0 {
                total_collected += claimable;
                stream.balance -= claimable;
                stream.last_collected = current_time;
                env.storage().persistent().set(&stream_key, &stream);
            }
        }

        // Single transfer of the total collected amount to the creator
        if total_collected > 0 {
            // All streams share the same token — read it from the first valid stream
            for i in 0..limit {
                let subscriber = subs.get(i as u32).unwrap();
                let stream_key = DataKey::Stream(subscriber.clone(), creator.clone());
                if env.storage().persistent().has(&stream_key) {
                    let stream: Stream = env.storage().persistent().get(&stream_key).unwrap();
                    let token_client = TokenClient::new(&env, &stream.token);
                    token_client.transfer(&env.current_contract_address(), &creator, &total_collected);
                    break;
                }
            }
        }

        total_collected
    }
}

mod test;
