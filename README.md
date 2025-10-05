# Honorary Quote Fee Distribution

Solana program for distributing quote-only fees from Meteora DAMM pools to Streamflow vesting investors.

## Overview

This program enables honorary positions that collect quote fees from CP-AMM pools and distribute them proportionally to investors based on their locked tokens in Streamflow vesting contracts. The distribution runs on a 24-hour cycle with configurable caps and minimum payouts.

## Setup

### Prerequisites

```bash
# Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"

# Anchor CLI
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install 0.31.1
avm use 0.31.1
```

### Build

```bash
anchor build
```

### Testing

```bash
# Run Rust unit tests
cargo test -p honorary_quote_fee -- --nocapture

# Start local validator with CP-AMM and Streamflow
./scripts/start-validator.sh

# Run integration tests (in another terminal)
npm install
npm test
```

## Program Structure

### Instructions

**1. `initialize_policy`**
- Creates distribution policy for a DAMM pool
- Validates quote-only configuration
- Sets investor share percentage, caps, and thresholds

**2. `configure_honorary_position`**
- Sets up PDA-owned position
- Creates quote treasury and base fee check accounts
- Links position NFT to policy

**3. `crank_quote_fee_distribution`**
- Claims fees from CP-AMM position via CPI
- Reads Streamflow contracts from remaining accounts
- Distributes proportionally to investors
- Routes remainder to creator
- Supports pagination for large investor lists

### Accounts

**Policy** (`seeds = ["policy", damm_pool]`)
- Stores pool configuration and distribution parameters
- References pool authority and vaults
- Tracks position and treasury accounts

**HonoraryPosition** (`seeds = ["honorary", policy]`)
- PDA that owns the position NFT
- Authority for treasury accounts
- Signs fee distribution transfers

**DistributionProgress** (`seeds = ["progress", damm_pool]`)
- Tracks current day's distribution state
- Manages pagination cursor
- Stores carry-forward amounts

## Wiring

### Integration with CP-AMM

```rust
// 1. Policy init reads pool account
let pool = DammPoolAccount::deserialize(&pool_data)?;
assert_quote_only_pool(&pool, quote_mint)?;

// 2. Crank claims fees via CPI
cp_amm::invoke_claim_position_fee(
    &cp_amm_program,
    &pool,
    &position,
    &base_vault,  // Must be unchanged
    &quote_vault, // Fee destination
    ...
)?;
```

### Integration with Streamflow

```rust
// Remaining accounts: [stream1, ata1, stream2, ata2, ...]
for chunk in remaining_accounts.chunks(2) {
    let stream = Stream::deserialize(&chunk[0].data)?;
    let locked = calculate_locked_amount(&stream, now)?;
    investors.push(InvestorEntry { locked, ata: chunk[1] });
}
```

### Fee Distribution Logic

```rust
// Compute eligible share based on total locked
let locked_ratio = total_locked / Y0;
let eligible_share_bps = min(investor_fee_share_bps, locked_ratio * 10000);

// Target distribution
let investor_target = claimed_fees * eligible_share_bps / 10000;
let investor_target = min(investor_target, daily_cap);

// Pro-rata distribution
for investor in investors {
    let payout = (investor_target * investor.locked) / total_locked;
    if payout >= min_payout {
        transfer(treasury, investor.ata, payout)?;
    }
}

// Creator gets remainder
let creator_amount = claimed_fees - distributed;
transfer(treasury, creator_ata, creator_amount)?;
```

## PDAs

| Account | Seeds | Description |
|---------|-------|-------------|
| Policy | `["policy", damm_pool]` | Distribution configuration |
| HonoraryPosition | `["honorary", policy]` | Fee collector and distributor |
| DistributionProgress | `["progress", damm_pool]` | Daily distribution state |

All PDAs use the Honorary Quote Fee program ID as the program_id parameter.

## Policies

### Distribution Parameters

- **investor_fee_share_bps**: Maximum investor share (0-10000 basis points)
- **y0**: Locked amount threshold for full investor share
- **daily_cap_quote**: Maximum distribution per day (0 = no cap)
- **min_payout_lamports**: Minimum payout per investor

### Eligibility

Investors must have active Streamflow vesting contracts with:
- Mint matching the policy's quote mint
- Non-zero locked amount at distribution time
- Valid recipient token account

### Distribution Schedule

1. **Day Open**: First crank after 24h opens new day
2. **Pages**: Process investors in batches (pagination)
3. **Day Close**: Last page distributes remainder to creator
4. **Next Day**: Cannot reopen until 24h elapsed

## Failure Modes

### Deterministic Failures (Transaction Reverts)

| Error | Condition | Resolution |
|-------|-----------|------------|
| `BaseFeeDetected` | Base fee > 0 in treasury | Wait for quote-only fees |
| `DayNotReady` | < 24h since last close | Wait for timer |
| `UnexpectedPageCursor` | Cursor mismatch | Use correct cursor |
| `InvalidInvestorShare` | Share > 10000 bps | Fix policy params |
| `Unauthorized` | Wrong authority | Use policy authority |
| `HonoraryPositionNotReady` | Position not configured | Call configure_honorary_position |

### Pool Validation Failures

- **Non-quote fees**: Policy creation fails if pool allows base fees
- **Partner pool**: Fails if pool has non-default partner
- **Mint mismatch**: Position NFT must match pool configuration

### Streamflow Parsing

- **Invalid mint**: Contracts with wrong mint are skipped
- **Deserialization error**: Invalid contracts are skipped
- **Zero locked**: Investors with 0 locked amount receive 0

### Dust Handling

Payouts below `min_payout_lamports` are:
1. Set to 0 for that investor
2. Accumulated in `progress.carry_quote`
3. Added to next distribution attempt
4. Routed to creator on day close if share = 0

## Events

- **HonoraryPositionInitialized**: Position setup complete
- **QuoteFeesClaimed**: Fees collected from CP-AMM
- **InvestorPayoutPage**: Page processed, amounts distributed
- **CreatorPayoutDayClosed**: Day closed, remainder to creator

## Test Coverage

The test suite includes:
- 24 Rust unit tests covering math, distribution logic, and edge cases
- Integration tests with CP-AMM and Streamflow on local validator
- Scenarios: partial locks, all unlocked, dust, caps, pagination

Run tests:
```bash
# Unit tests
cargo test -p honorary_quote_fee

# Integration tests  
./scripts/start-validator.sh  # Terminal 1
npm test                        # Terminal 2
```

## License

MIT
