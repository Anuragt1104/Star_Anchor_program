# Honorary Quote Fee Program

A Solana program that implements honorary quote fee distribution for Meteora DAMM (Dynamic Automated Market Maker) pools with Streamflow vesting integration.

## Overview

This program allows pool creators to set up honorary positions that collect quote-only fees from DAMM pools and distribute them to investors based on their locked tokens in Streamflow vesting contracts. The system includes:

- **Policy Management**: Configure fee distribution parameters for quote-only pools
- **Honorary Positions**: Special positions that collect and distribute fees
- **Fee Distribution**: Proportional distribution to investors based on locked amounts
- **Streamflow Integration**: Automatic investor detection from vesting contracts
- **Daily Limits**: Configurable daily caps and minimum payout thresholds

## Architecture

### Components

1. **Policy Account**: Stores distribution parameters and pool configuration
2. **Honorary Position**: PDA that holds collected fees and executes distributions
3. **Distribution Progress**: Tracks daily distribution state and progress
4. **Treasury Accounts**: Hold collected fees before distribution

### Key Features

- **Quote-Only Pools**: Only distributes quote token fees (no base token fees)
- **Proportional Distribution**: Investors receive fees proportional to their locked amounts
- **Daily Cycling**: Distributions happen daily with configurable caps
- **Minimum Thresholds**: Ensures meaningful payouts to investors
- **Creator Share**: Configurable split between investors and pool creator

## End-to-End Testing

The test suite demonstrates complete flows against CP-AMM and Streamflow on a local validator.

### Test Coverage

1. **Policy Initialization**
   - Setup distribution parameters
   - Configure pool and treasury accounts
   - Validate parameter constraints

2. **Honorary Position Configuration**
   - Create position NFT and accounts
   - Setup treasury and fee checking accounts
   - Enable fee collection

3. **Fee Accumulation**
   - Simulate trading fees from CP-AMM
   - Monitor treasury balance changes

4. **Fee Distribution**
   - Parse Streamflow vesting contracts
   - Calculate proportional shares
   - Execute token transfers
   - Handle daily caps and minimum payouts

### Running Tests

#### Prerequisites

```bash
# Install dependencies
npm install

# Install Solana CLI and Anchor
curl -sSfL https://install.solana.com | sh
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

#### Local Testing

```bash
# Run comprehensive Rust unit tests with detailed output
cargo test -p honorary_quote_fee -- --nocapture --test-threads=1

# Run specific test category
cargo test -p honorary_quote_fee math -- --nocapture
cargo test -p honorary_quote_fee payout_plan -- --nocapture
cargo test -p honorary_quote_fee integration -- --nocapture

# Start local validator with programs
npm run validator

# In another terminal, run tests
npm test

# Or run end-to-end test with validator
npm run test:e2e
```

**Rust Test Results:**
```
running 24 tests

✅ 4 Math operation tests - All passed
✅ 2 Share calculation tests - All passed
✅ 13 Payout distribution tests - All passed
✅ 2 Integration scenario tests - All passed
✅ 1 Requirements verification test - All passed

test result: ok. 24 passed; 0 failed; 0 ignored
```

#### Test Structure

```
programs/honorary_quote_fee/src/
└── tests.rs              # Comprehensive Rust unit tests (24 tests)

tests/
├── helpers/
│   └── setup.ts          # Test environment utilities
└── honorary_quote_fee.ts # End-to-end test suite

TESTING.md                # Detailed test documentation
```

### Test Scenarios

1. **Complete Flow Test**
   - Initialize policy for quote-only pool
   - Configure honorary position
   - Simulate fee accumulation
   - Execute distribution to investors
   - Verify proportional payouts

2. **Integration Tests**
   - Multiple day distributions
   - Daily cap enforcement
   - Minimum payout thresholds
   - Streamflow contract parsing

3. **Error Handling**
   - Invalid policy parameters
   - Unauthorized operations
   - Insufficient funds
   - Pool validation failures

## Program Interface

### Instructions

#### Initialize Policy
```rust
initialize_policy(params: InitializePolicyParams)
```
Creates a new fee distribution policy for a DAMM pool.

**Parameters:**
- `investor_fee_share_bps`: Basis points for investor share (0-10000)
- `y0`: Minimum locked amount threshold
- `daily_cap_quote`: Maximum daily distribution
- `min_payout_lamports`: Minimum individual payout

#### Configure Honorary Position
```rust
configure_honorary_position()
```
Sets up the honorary position and treasury accounts.

#### Crank Quote Fee Distribution
```rust
crank_quote_fee_distribution(params: CrankQuoteFeeParams)
```
Executes fee collection and distribution.

**Parameters:**
- `expected_page_cursor`: Pagination cursor
- `max_page_cursor`: Maximum pages to process
- `is_last_page`: Whether this completes the day's distribution

## Building and Deployment

```bash
# Build the program
anchor build

# Deploy to localnet
anchor deploy

# Test on localnet
anchor test
```

## Dependencies

- **Anchor**: Solana framework for Rust programs
- **Meteora DAMM**: Decentralized exchange protocol
- **Streamflow**: Token vesting and locking protocol
- **SPL Token**: Solana token standard

## Security Considerations

- **Access Control**: Only authorized accounts can modify policies
- **Validation**: Extensive input validation and pool verification
- **Fee Limits**: Daily caps prevent excessive distributions
- **Auditability**: All distributions are logged with events

## Development

### Project Structure
```
programs/honorary_quote_fee/
├── src/
│   ├── lib.rs           # Main program logic
│   ├── cp_amm.rs        # CP-AMM integration
│   ├── streamflow_utils.rs # Streamflow parsing
│   ├── state.rs         # Account structures
│   ├── errors.rs        # Error definitions
│   └── events.rs        # Event logging
├── tests/               # Test suite
└── Cargo.toml          # Dependencies
```

### Key Components

- **CP-AMM Integration**: Handles fee claiming from DAMM positions
- **Streamflow Utils**: Parses vesting contracts to identify investors
- **Math Utils**: Safe arithmetic for fee calculations
- **State Management**: Account structures with proper constraints

## License

MIT
