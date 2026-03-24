# SubStream Protocol: Decentralized Creator Economy

## Overview

SubStream is a **Pay-As-You-Go subscription protocol**.
Instead of monthly credit card charges, fans stream tokens to creators second-by-second.
If the fan dislikes the content, they can cancel instantly and get their remaining balance back.

## Key Logic

- **subscribe**: User deposits a buffer (e.g., 50 XLM) and sets a rate.
- **collect**: Creator triggers the withdrawal of accumulated seconds.
- **cancel**: Subscriber stops the stream and refunds unspent tokens (after minimum duration).

## Sybil Protection

To prevent users from rapidly starting/stopping streams to "scrape" content, the protocol enforces a **minimum flow duration of 24 hours**. Once a stream is initiated, it cannot be canceled until the minimum duration has elapsed. This protects creators from abuse and ensures meaningful engagement.
- **cancel**: Subscriber stops the stream and refunds unspent tokens.
- **subscribe_group**: User streams to a group channel with exactly 5 creators and percentage splits that sum to 100.
- **collect_group**: Contract automatically splits each collected amount to all 5 creators based on configured percentages.

## Network

- **Stellar Testnet**

## Deployed Contract

- **Network:** Stellar Testnet
- **Contract ID:** CAOUX2FZ65IDC4F2X7LJJ2SVF23A35CCTZB7KVVN475JCLKTTU4CEY6L

## Subscription State Flow

```mermaid
stateDiagram-v2
    [*] --> Trial : subscribe()

    Trial --> Active : trial period ends\n(stream begins)
    Trial --> Expired : cancel() during trial

    Active --> GracePeriod : balance runs low\n(below rate threshold)
    Active --> Expired : cancel()

    GracePeriod --> Active : top_up() refills balance
    GracePeriod --> Expired : balance depleted\nor cancel()

    Expired --> [*]
```

## Running Tests

To run the contract tests locally:

```bash
cargo test
```

## Building

To build the contract for Wasm:

```bash
cargo build --target wasm32-unknown-unknown --release
```
