#![no_std]
use soroban_sdk::contractevent;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, vec, Address, Bytes, Env, Vec};

// Minimum flow duration: 24 hours in seconds (24 * 60 * 60 = 86400)
const MINIMUM_FLOW_DURATION: u64 = 86400;
const FREE_TRIAL_DURATION: u64 = 7 * 24 * 60 * 60;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Stream(Address, Address),        // (subscriber, creator)
    TotalStreamed(Address, Address), // (subscriber, creator) - cumulative tokens streamed
    CliffThreshold(Address),         // creator -> threshold amount for access
    CreatorSubscribers(Address),     // creator -> Vec<subscriber>
    CreatorMetadata(Address),        // creator -> IPFS CID bytes
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

#[contractevent]
pub struct TipReceived {
    #[topic]
    pub user: Address,
    #[topic]
    pub creator: Address,
    #[topic]
    pub token: Address,
    pub amount: i128,
}

#[contract]
pub struct SubStreamContract;

fn stream_key(subscriber: &Address, stream_id: &Address) -> DataKey {
    DataKey::Stream(subscriber.clone(), stream_id.clone())
}

fn validate_distribution(creators: &Vec<Address>, percentages: &Vec<u32>, expected: u32) {
    let creators_len = creators.len();
    if creators_len != expected {
        panic!("creator count mismatch");
    }
    
    let mut total_percentage: u32 = 0;
    for i in 0..creators_len {
        let percentage = percentages.get(i as u32).unwrap();
        total_percentage += percentage;
    }
    
    if total_percentage != 100 {
        panic!("percentages must sum to 100");
    }
}

fn stream_exists(env: &Env, key: &DataKey) -> bool {
    env.storage().persistent().has(key) || env.storage().temporary().has(key)
}

fn get_stream(env: &Env, key: &DataKey) -> Stream {
    if env.storage().persistent().has(key) {
        env.storage().persistent().get(key).unwrap()
    } else if env.storage().temporary().has(key) {
        env.storage().temporary().get(key).unwrap()
    } else {
        panic!("stream not found")
    }
}

fn set_stream(env: &Env, key: &DataKey, stream: &Stream) {
    if stream.balance > 0 {
        env.storage().persistent().set(key, stream);
        env.storage().temporary().remove(key);
    } else {
        env.storage().temporary().set(key, stream);
        env.storage().persistent().remove(key);
    }
}

fn remove_stream(env: &Env, key: &DataKey) {
    env.storage().persistent().remove(key);
    env.storage().temporary().remove(key);
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
        subscriber.require_auth();

        if amount <= 0 || rate_per_second <= 0 {
            panic!("amount and rate must be positive");
        }

        let key = stream_key(&subscriber, &creator);
        if stream_exists(&env, &key) {
            panic!("stream already exists");
        }

        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&subscriber, &env.current_contract_address(), &amount);

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
            creators: vec![&env, creator.clone()],
            percentages: vec![&env, 100],
        };

        env.storage().persistent().set(&key, &stream);

        add_subscriber_to_creator(&env, &creator, &subscriber);
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
        if stream.tier.rate_per_second <= 0 || stream.balance <= 0 {
            return false;
        }

        let trial_end = stream
            .start_time
            .saturating_add(stream.tier.trial_duration);
        let charge_start = if stream.last_collected > trial_end {
            stream.last_collected
        } else {
            trial_end
        };

        let now = env.ledger().timestamp();
        if now <= charge_start {
            return true;
        }

        let elapsed = (now - charge_start) as i128;
        let potential_charge = elapsed
            .checked_mul(stream.tier.rate_per_second)
            .unwrap_or(0);
        
        stream.balance > potential_charge
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
            let s_key = stream_key(&subscriber, &creator);
            if stream_exists(&env, &s_key) {
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
            if stream_exists(&env, &s_key) {
                let mut stream = get_stream(&env, &s_key);
                stream.last_collected = now;
                set_stream(&env, &s_key, &stream);
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
    ) {
        subscriber.require_auth();

        let key = stream_key(&subscriber, &creator);
        if !stream_exists(&env, &key) {
            panic!("stream not found");
        }

        let mut stream = get_stream(&env, &key);
        let old_rate = stream.tier.rate_per_second;
        
        if old_rate == new_rate_per_second {
            return;
        }

        // Collect any pending earnings before changing rate
        distribute_and_collect(&env, &subscriber, &creator, Some(&creator));
        stream = get_stream(&env, &key);

        stream.tier.rate_per_second = new_rate_per_second;
        set_stream(&env, &key, &stream);

        env.events().publish(
            TierChanged {
                subscriber: subscriber.clone(),
                creator: creator.clone(),
                old_rate,
                new_rate: new_rate_per_second,
            }
        );
    }

    /// Collect from all active streams for a creator in a single call.
    /// `max_count` caps the batch size to avoid hitting ledger instruction limits.
    /// Returns the total amount collected across all processed streams.
    pub fn withdraw_all(env: Env, creator: Address, max_count: u32) -> i128 {
        let subs_key = DataKey::CreatorSubscribers(creator.clone());
        let subs: Vec<Address> = env.storage().persistent().get(&subs_key).unwrap_or(vec![&env]);

        let mut total: i128 = 0;
        let limit = max_count.min(subs.len());

        for i in 0..limit {
            let subscriber = subs.get(i).unwrap();
            let s_key = stream_key(&subscriber, &creator);
            if stream_exists(&env, &s_key) {
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

    /// Store an IPFS CID pointing to the creator's profile, links, and tier descriptions.
    /// Only the creator themselves can update their own metadata.
    pub fn set_creator_metadata(env: Env, creator: Address, cid: Bytes) {
        creator.require_auth();
        let key = DataKey::CreatorMetadata(creator.clone());
        env.storage().persistent().set(&key, &cid);
    }

    /// Retrieve the IPFS CID for a creator. Returns None if not set.
    pub fn get_creator_metadata(env: Env, creator: Address) -> Option<Bytes> {
        let key = DataKey::CreatorMetadata(creator.clone());
        env.storage().persistent().get(&key)
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

    /// Direct tip from user to creator without subscription
    /// Transfers tokens directly from user to creator and emits TipReceived event
    pub fn tip(env: Env, user: Address, creator: Address, token: Address, amount: i128) {
        user.require_auth();
        
        if amount <= 0 {
            panic!("amount must be positive");
        }
        
        if user == creator {
            panic!("cannot tip yourself");
        }
        
        // Direct transfer from user to creator
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&user, &creator, &amount);
        
        // Emit TipReceived event
        env.events().publish(
            (user.clone(), creator.clone(), token.clone()),
            amount,
        );
    }

    // Update total streamed amount for a subscriber-creator pair
    fn update_total_streamed(env: &Env, subscriber: &Address, creator: &Address, amount: i128) {
        let key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
        let current_total: i128 = env.storage().persistent().get(&key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&key, &(current_total + amount));
    }
}

// Helper functions
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

fn distribute_and_collect(
    env: &Env,
    subscriber: &Address,
    stream_id: &Address,
    total_streamed_creator: Option<&Address>,
) -> i128 {
    let key = stream_key(subscriber, stream_id);
    let mut stream = get_stream(env, &key);
    let now = env.ledger().timestamp();

    if now <= stream.last_collected {
        return 0;
    }

    if let Some(creator) = total_streamed_creator {
        if is_creator_paused(env, creator) {
            // While paused, advance accounting clock so paused time is never billed.
            stream.last_collected = now;
            set_stream(env, &key, &stream);
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
        .unwrap_or(0);

    if amount_to_collect > stream.balance {
        amount_to_collect = stream.balance;
    }

    if amount_to_collect <= 0 {
        return 0;
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
    stream.last_collected = now;
    set_stream(env, &key, &stream);

    // Update cumulative streamed for each creator
    for i in 0..stream.creators.len() {
        let creator = stream.creators.get(i).unwrap();
        SubStreamContract::update_total_streamed(env, subscriber, &creator, amount_to_collect);
    }

    amount_to_collect
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
        .checked_mul(stream.tier.rate_per_second)
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

fn update_total_streamed(env: &Env, subscriber: &Address, creator: &Address, amount: i128) {
    let key = DataKey::TotalStreamed(subscriber.clone(), creator.clone());
    let current_total: i128 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage()
        .persistent()
        .set(&key, &(current_total + amount));
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

fn cancel_group_internal(env: &Env, subscriber: &Address, stream_id: &Address) {
    subscriber.require_auth();

    let key = stream_key(subscriber, stream_id);
    if !stream_exists(env, &key) {
        panic!("stream not found");
    }

    // Check minimum flow duration
    let stream: Stream = env.storage().persistent().get(&key).unwrap();
    let current_time = env.ledger().timestamp();
    if current_time < stream.start_time + MINIMUM_FLOW_DURATION {
        let remaining_time = stream.start_time + MINIMUM_FLOW_DURATION - current_time;
        panic!(
            "cannot cancel stream: minimum duration not met. {} seconds remaining",
            remaining_time
        );
    }

    collect_internal(env, subscriber, stream_id);
    distribute_and_collect(env, subscriber, stream_id, None);

    let stream = get_stream(env, &key);
    if stream.balance > 0 {
        let token_client = TokenClient::new(env, &stream.token);
        token_client.transfer(&env.current_contract_address(), subscriber, &stream.balance);
    }

    remove_stream(env, &key);

    // Remove subscriber from stream_id's subscriber list
    let creator_key = DataKey::CreatorSubscribers(stream_id.clone());
    if let Some(subs) = env
        .storage()
        .persistent()
        .get::<DataKey, Vec<Address>>(&creator_key)
    {
        let mut updated: Vec<Address> = vec![env];
        for s in subs.iter() {
            if s != *subscriber {
                updated.push_back(s);
            }
        }
        env.storage().persistent().set(&creator_key, &updated);
    }
    remove_stream(env, &key);
}

mod test;
