#![no_std]

use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{
    contract, contractevent, contractimpl, contracttype, vec, Address, Env, Vec,
};

// Minimum flow duration: 24 hours in seconds (24 * 60 * 60 = 86400)
const MINIMUM_FLOW_DURATION: u64 = 86400;
const FREE_TRIAL_DURATION: u64 = 7 * 24 * 60 * 60;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address),        // (subscriber, stream_id)
    TotalStreamed(Address, Address), // (subscriber, creator)
    CliffThreshold(Address),         // creator -> threshold amount
    CreatorSubscribers(Address),     // creator -> Vec<subscriber>
    ChannelPaused(Address),          // creator -> bool
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tier {
    pub rate_per_second: i128,
    pub trial_duration: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Stream {
    pub token: Address,
    pub tier: Tier,
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
    let len = creators.len();
    for i in 0..len {
        let percentage = percentages.get(i).unwrap();
        if percentage == 0 {
            panic!("percentages must be positive");
        }
        total = total.checked_add(percentage).expect("overflow");

        let creator_i = creators.get(i).unwrap();
        for j in (i + 1)..len {
            if creator_i == creators.get(j).unwrap() {
                panic!("creators must be unique");
            }
        }
    }

    if total != 100 {
        panic!("percentages must sum to 100");
    }
}

fn is_creator_paused(env: &Env, creator: &Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::ChannelPaused(creator.clone()))
        .unwrap_or(false)
}

fn add_subscriber_to_creator(env: &Env, creator: &Address, subscriber: &Address) {
    let key = DataKey::CreatorSubscribers(creator.clone());
    let mut subs: Vec<Address> = env.storage().persistent().get(&key).unwrap_or(vec![env]);

    for s in subs.iter() {
        if s == *subscriber {
            return;
        }
    }

    subs.push_back(subscriber.clone());
    env.storage().persistent().set(&key, &subs);
}

fn remove_subscriber_from_creator(env: &Env, creator: &Address, subscriber: &Address) {
    let key = DataKey::CreatorSubscribers(creator.clone());
    let subs: Vec<Address> = env.storage().persistent().get(&key).unwrap_or(vec![env]);

    let mut updated = vec![env];
    for s in subs.iter() {
        if s != *subscriber {
            updated.push_back(s);
        }
    }

    env.storage().persistent().set(&key, &updated);
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

    let now = env.ledger().timestamp();
    let stream = Stream {
        token: token.clone(),
        tier: Tier {
            rate_per_second,
            trial_duration: FREE_TRIAL_DURATION,
        },
        balance: amount,
        last_collected: now,
        start_time: now,
        creators,
        percentages,
    };

    env.storage().persistent().set(&key, &stream);
}

fn distribute_and_collect(
    env: &Env,
    subscriber: &Address,
    stream_id: &Address,
    total_streamed_creator: Option<&Address>,
) -> i128 {
    let key = stream_key(subscriber, stream_id);
    if !env.storage().persistent().has(&key) {
        panic!("stream not found");
    }

    let mut stream: Stream = env.storage().persistent().get(&key).unwrap();
    let now = env.ledger().timestamp();

    if now <= stream.last_collected {
        return 0;
    }

    if let Some(creator) = total_streamed_creator {
        if is_creator_paused(env, creator) {
            // While paused, advance accounting clock so paused time is never billed.
            stream.last_collected = now;
            env.storage().persistent().set(&key, &stream);
            return 0;
        }
    }

    let trial_end = stream
        .start_time
        .saturating_add(stream.tier.trial_duration);
    let charge_start = if stream.last_collected > trial_end {
        stream.last_collected
    } else {
        trial_end
    };

    if now <= charge_start {
        return 0;
    }

    let elapsed = (now - charge_start) as i128;
    let mut amount_to_collect = elapsed
        .checked_mul(stream.tier.rate_per_second)
        .expect("overflow");

    if amount_to_collect > stream.balance {
        amount_to_collect = stream.balance;
    }

    if amount_to_collect <= 0 {
        return 0;
    }

    let token_client = TokenClient::new(env, &stream.token);
    let mut remaining = amount_to_collect;
    let len = stream.creators.len();

    for i in 0..len {
        let creator = stream.creators.get(i).unwrap();
        let payout = if i + 1 == len {
            remaining
        } else {
            let percentage = stream.percentages.get(i).unwrap() as i128;
            let split = amount_to_collect
                .checked_mul(percentage)
                .expect("overflow")
                .checked_div(100)
                .expect("div by zero");
            remaining -= split;
            split
        };

        if payout > 0 {
            token_client.transfer(&env.current_contract_address(), &creator, &payout);
        }
    }

    stream.balance -= amount_to_collect;
    stream.last_collected = now;
    env.storage().persistent().set(&key, &stream);

    if let Some(creator) = total_streamed_creator {
        let total_key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        let total: i128 = env.storage().persistent().get(&total_key).unwrap_or(0);
        let new_total = total.checked_add(amount_to_collect).expect("overflow");
        env.storage().persistent().set(&total_key, &new_total);
    }

    amount_to_collect
}

fn cancel_group_internal(env: &Env, subscriber: &Address, stream_id: &Address) {
    subscriber.require_auth();

    let key = stream_key(subscriber, stream_id);
    if !env.storage().persistent().has(&key) {
        panic!("stream not found");
    }

    distribute_and_collect(env, subscriber, stream_id, None);

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

    stream.balance = stream.balance.checked_add(amount).expect("overflow");
    env.storage().persistent().set(&key, &stream);
}

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

        add_subscriber_to_creator(&env, &creator, &subscriber);
    }

    pub fn collect(env: Env, subscriber: Address, creator: Address) {
        distribute_and_collect(&env, &subscriber, &creator, Some(&creator));
    }

    pub fn cancel(env: Env, subscriber: Address, creator: Address) {
        subscriber.require_auth();

        let key = stream_key(&subscriber, &creator);
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        let stream: Stream = env.storage().persistent().get(&key).unwrap();
        let now = env.ledger().timestamp();
        if now < stream.start_time + MINIMUM_FLOW_DURATION {
            panic!("cannot cancel stream: minimum duration not met");
        }

        distribute_and_collect(&env, &subscriber, &creator, Some(&creator));

        let stream_after: Stream = env.storage().persistent().get(&key).unwrap();
        if stream_after.balance > 0 {
            let token_client = TokenClient::new(&env, &stream_after.token);
            token_client.transfer(
                &env.current_contract_address(),
                &subscriber,
                &stream_after.balance,
            );
        }

        env.storage().persistent().remove(&key);
        remove_subscriber_from_creator(&env, &creator, &subscriber);
    }

    pub fn top_up(env: Env, subscriber: Address, creator: Address, amount: i128) {
        top_up_internal(&env, &subscriber, &creator, amount);
    }

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
        distribute_and_collect(&env, &subscriber, &channel_id, None);
    }

    pub fn cancel_group(env: Env, subscriber: Address, channel_id: Address) {
        cancel_group_internal(&env, &subscriber, &channel_id);
    }

    pub fn top_up_group(env: Env, subscriber: Address, channel_id: Address, amount: i128) {
        top_up_internal(&env, &subscriber, &channel_id, amount);
    }

    /// Creator-level pause: stops charging all incoming streams for this creator.
    pub fn pause_channel(env: Env, creator: Address) {
        creator.require_auth();

        if is_creator_paused(&env, &creator) {
            return;
        }

        let key = DataKey::CreatorSubscribers(creator.clone());
        let subs: Vec<Address> = env.storage().persistent().get(&key).unwrap_or(vec![&env]);

        // Settle all streams up to pause timestamp, then freeze charging.
        for subscriber in subs.iter() {
            if env
                .storage()
                .persistent()
                .has(&stream_key(&subscriber, &creator))
            {
                distribute_and_collect(&env, &subscriber, &creator, Some(&creator));
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::ChannelPaused(creator), &true);
    }

    pub fn unpause_channel(env: Env, creator: Address) {
        creator.require_auth();

        if !is_creator_paused(&env, &creator) {
            return;
        }

        let key = DataKey::CreatorSubscribers(creator.clone());
        let subs: Vec<Address> = env.storage().persistent().get(&key).unwrap_or(vec![&env]);
        let now = env.ledger().timestamp();

        // Resume billing from now so paused window is never charged.
        for subscriber in subs.iter() {
            let s_key = stream_key(&subscriber, &creator);
            if env.storage().persistent().has(&s_key) {
                let mut stream: Stream = env.storage().persistent().get(&s_key).unwrap();
                stream.last_collected = now;
                env.storage().persistent().set(&s_key, &stream);
            }
        }

        env.storage()
            .persistent()
            .set(&DataKey::ChannelPaused(creator), &false);
    }

    pub fn is_channel_paused(env: Env, creator: Address) -> bool {
        is_creator_paused(&env, &creator)
    }

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

        let key = stream_key(&subscriber, &creator);
        if !env.storage().persistent().has(&key) {
            panic!("stream not found");
        }

        let stream_before: Stream = env.storage().persistent().get(&key).unwrap();
        let old_rate = stream_before.tier.rate_per_second;

        distribute_and_collect(&env, &subscriber, &creator, Some(&creator));

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

        stream.tier.rate_per_second = new_rate_per_second;
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

    /// Collect from a creator's active streams in a batch.
    pub fn withdraw_all(env: Env, creator: Address, max_count: u32) -> i128 {
        let subs_key = DataKey::CreatorSubscribers(creator.clone());
        let subs: Vec<Address> = env.storage().persistent().get(&subs_key).unwrap_or(vec![&env]);

        let mut total: i128 = 0;
        let limit = max_count.min(subs.len());

        for i in 0..limit {
            let subscriber = subs.get(i).unwrap();
            if env
                .storage()
                .persistent()
                .has(&stream_key(&subscriber, &creator))
            {
                total += distribute_and_collect(&env, &subscriber, &creator, Some(&creator));
            }
        }

        total
    }

    pub fn set_cliff_threshold(env: Env, creator: Address, threshold: i128) {
        creator.require_auth();

        if threshold < 0 {
            panic!("threshold must be non-negative");
        }

        env.storage()
            .persistent()
            .set(&DataKey::CliffThreshold(creator), &threshold);
    }

    pub fn get_cliff_threshold(env: Env, creator: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::CliffThreshold(creator))
            .unwrap_or(0)
    }

    pub fn get_total_streamed(env: Env, subscriber: Address, creator: Address) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::TotalStreamed(subscriber, creator))
            .unwrap_or(0)
    }

    pub fn has_unlocked_access(env: Env, subscriber: Address, creator: Address) -> bool {
        let threshold: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::CliffThreshold(creator.clone()))
            .unwrap_or(0);

        if threshold == 0 {
            return true;
        }

        let total_streamed: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalStreamed(subscriber, creator))
            .unwrap_or(0);

        total_streamed >= threshold
    }

    pub fn get_access_tier(env: Env, subscriber: Address, creator: Address) -> u32 {
        let total_streamed: i128 = env
            .storage()
            .persistent()
            .get(&DataKey::TotalStreamed(subscriber, creator))
            .unwrap_or(0);

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
}

mod test;
