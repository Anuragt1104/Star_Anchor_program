# Instruction Interface Reference

Complete reference for all program instructions, parameters, and account requirements.

---

## Program ID

```
7YupTAYp9uHuv5UJdGGVfX1dr1WNd71ezW43r3UxbxMk
```

---

## Instructions

### 1. `initialize_policy`

Creates a new fee distribution policy for a DAMM pool.

#### Parameters

```rust
pub struct InitializePolicyParams {
    pub investor_fee_share_bps: u16,  // Max investor share (0-10000 BPS)
    pub y0: u64,                       // Locked threshold for full share
    pub daily_cap_quote: u64,          // Daily distribution cap (0 = no cap)
    pub min_payout_lamports: u64,      // Minimum payout per investor
}
```

**Field Descriptions:**
- `investor_fee_share_bps`: Maximum basis points (0-10000) allocated to investors. If locked amount < y0, actual share is reduced proportionally.
- `y0`: Minimum locked amount threshold. When locked = y0, investors get full share.
- `daily_cap_quote`: Maximum quote tokens distributed per day. Set to 0 for unlimited.
- `min_payout_lamports`: Minimum payout amount. Smaller payouts accumulate as dust.

**Constraints:**
- `investor_fee_share_bps` ≤ 10000
- `y0` > 0

#### Accounts

| Account | Type | Mutable | Signer | Seeds | Description |
|---------|------|---------|--------|-------|-------------|
| `payer` | `Signer` | ✓ | ✓ | - | Pays for account creation |
| `authority` | `Signer` | ✓ | ✓ | - | Policy authority (can modify policy) |
| `policy` | `Account<Policy>` | ✓ | - | `["policy", damm_pool]` | Policy account (initialized) |
| `progress` | `Account<DistributionProgress>` | ✓ | - | `["progress", damm_pool]` | Progress tracker (initialized) |
| `damm_pool` | `UncheckedAccount` | ✓ | - | - | CP-AMM pool account |
| `pool_authority` | `UncheckedAccount` | - | - | - | CP-AMM pool authority PDA |
| `damm_program` | `UncheckedAccount` | - | - | - | CP-AMM program ID |
| `quote_mint` | `Account<Mint>` | - | - | - | Quote token mint |
| `base_mint` | `Account<Mint>` | - | - | - | Base token mint |
| `quote_vault` | `Account<TokenAccount>` | ✓ | - | - | Pool's quote vault |
| `base_vault` | `Account<TokenAccount>` | ✓ | - | - | Pool's base vault |
| `creator_quote_ata` | `Account<TokenAccount>` | ✓ | - | - | Creator's quote token account |
| `system_program` | `Program<System>` | - | - | - | System program |

#### Validation

The instruction performs the following validation:
1. Deserializes and validates CP-AMM pool structure
2. Ensures pool is quote-only (no base fees)
3. Verifies pool has no partner set
4. Validates mint addresses match pool configuration
5. Checks vault addresses match pool

---

### 2. `configure_honorary_position`

Sets up the honorary position and treasury accounts.

#### Parameters

None

#### Accounts

| Account | Type | Mutable | Signer | Seeds | Description |
|---------|------|---------|--------|-------|-------------|
| `authority` | `Signer` | ✓ | ✓ | - | Must match policy authority |
| `policy` | `Account<Policy>` | ✓ | - | `["policy", damm_pool]` | Policy account |
| `honorary_position` | `Account<HonoraryPosition>` | ✓ | - | `["honorary", policy]` | Honorary position PDA (initialized) |
| `position` | `UncheckedAccount` | ✓ | - | - | Existing CP-AMM position |
| `position_nft_mint` | `Account<Mint>` | - | - | - | Position NFT mint (decimals = 0) |
| `position_nft_account` | `Account<TokenAccount>` | ✓ | - | - | Position NFT account (owner = honorary_position) |
| `quote_mint` | `Account<Mint>` | - | - | - | Quote token mint |
| `quote_treasury` | `Account<TokenAccount>` | ✓ | - | ATA | Quote fee treasury (init_if_needed) |
| `base_mint` | `Account<Mint>` | - | - | - | Base token mint |
| `base_fee_check` | `Account<TokenAccount>` | ✓ | - | ATA | Base fee check account (init_if_needed) |
| `system_program` | `Program<System>` | - | - | - | System program |
| `token_program` | `Program<Token>` | - | - | - | Token program |
| `associated_token_program` | `Program<AssociatedToken>` | - | - | - | Associated token program |

**ATA**: Associated Token Account derived for the honorary_position PDA.

#### Validation

1. Deserializes and validates CP-AMM position structure
2. Ensures position belongs to the configured pool
3. Verifies position has no unclaimed fees
4. Checks position has no liquidity (clean position)
5. Validates position NFT (decimals = 0, amount = 1)
6. Ensures NFT is owned by honorary_position PDA
7. Verifies treasury accounts match mint and are owned by PDA

---

### 3. `crank_quote_fee_distribution`

Executes fee collection and distribution.

#### Parameters

```rust
pub struct CrankQuoteFeeParams {
    pub expected_page_cursor: u32,  // Expected pagination cursor
    pub max_page_cursor: u32,       // Maximum cursor value (0 = unlimited)
    pub is_last_page: bool,         // True to close the day
}
```

**Field Descriptions:**
- `expected_page_cursor`: Must match current `progress.page_cursor`. Ensures idempotency.
- `max_page_cursor`: Safety limit for pagination. Set to 0 for no limit.
- `is_last_page`: When true, distributes remainder to creator and closes the day.

#### Accounts

| Account | Type | Mutable | Signer | Seeds | Description |
|---------|------|---------|--------|-------|-------------|
| `cranker` | `UncheckedAccount` | - | ✓ | - | Transaction signer (anyone) |
| `policy` | `Account<Policy>` | ✓ | - | `["policy", damm_pool]` | Policy account |
| `honorary_position` | `Account<HonoraryPosition>` | ✓ | - | `["honorary", policy]` | Honorary position PDA |
| `progress` | `UncheckedAccount` | ✓ | - | `["progress", damm_pool]` | Distribution progress |
| `quote_treasury` | `UncheckedAccount` | ✓ | - | - | Quote fee treasury |
| `base_fee_check` | `UncheckedAccount` | ✓ | - | - | Base fee check account |
| `creator_quote_ata` | `UncheckedAccount` | ✓ | - | - | Creator's quote ATA |
| `pool` | `UncheckedAccount` | - | - | - | CP-AMM pool |
| `pool_authority` | `UncheckedAccount` | - | - | - | CP-AMM pool authority |
| `position` | `UncheckedAccount` | ✓ | - | - | CP-AMM position |
| `position_nft_account` | `UncheckedAccount` | ✓ | - | - | Position NFT account |
| `base_vault` | `UncheckedAccount` | ✓ | - | - | Pool's base vault |
| `quote_vault` | `UncheckedAccount` | ✓ | - | - | Pool's quote vault |
| `base_mint` | `UncheckedAccount` | - | - | - | Base token mint |
| `quote_mint` | `UncheckedAccount` | - | - | - | Quote token mint |
| `event_authority` | `UncheckedAccount` | - | - | - | CP-AMM event authority |
| `cp_amm_program` | `UncheckedAccount` | - | - | - | CP-AMM program ID |
| `token_program_a` | `UncheckedAccount` | - | - | - | Token program for base |
| `token_program_b` | `UncheckedAccount` | - | - | - | Token program for quote |
| `token_program` | `Program<Token>` | - | - | - | Token program |

#### Remaining Accounts

The instruction accepts remaining accounts in pairs:
```
[streamflow_contract_1, investor_1_ata,
 streamflow_contract_2, investor_2_ata,
 ...]
```

Each pair consists of:
1. **Streamflow Contract** (read-only): Vesting contract account
2. **Investor ATA** (writable): Investor's quote token account

**Requirements:**
- Must be passed in pairs
- Contract must have matching quote mint
- ATA must be for the correct recipient and mint

#### Execution Flow

1. **Day Opening** (if not open):
   - Verifies 24h elapsed since last close
   - Opens new day, resets counters

2. **Fee Collection**:
   - Claims fees from CP-AMM position via CPI
   - Records quote fees claimed
   - Ensures base fees = 0 (fails if any base fees)

3. **Investor Processing**:
   - Parses Streamflow contracts from remaining accounts
   - Calculates locked amounts at current timestamp
   - Computes eligible share based on total locked
   - Distributes proportionally to investors
   - Filters payouts below minimum threshold (dust)

4. **Day Closing** (if `is_last_page = true`):
   - Transfers remainder to creator
   - Emits day closed event
   - Resets day state

---

## Account Structures

### Policy

```rust
pub struct Policy {
    pub authority: Pubkey,           // Can modify policy
    pub pool: Pubkey,                // CP-AMM pool
    pub pool_authority: Pubkey,      // Pool authority PDA
    pub cp_amm_program: Pubkey,      // CP-AMM program ID
    pub quote_mint: Pubkey,          // Quote token mint
    pub base_mint: Pubkey,           // Base token mint
    pub quote_vault: Pubkey,         // Pool quote vault
    pub base_vault: Pubkey,          // Pool base vault
    pub position: Pubkey,            // CP-AMM position
    pub position_nft_mint: Pubkey,   // Position NFT mint
    pub position_nft_account: Pubkey,// Position NFT account
    pub quote_treasury: Pubkey,      // Quote fee treasury
    pub base_fee_check: Pubkey,      // Base fee check account
    pub creator_quote_ata: Pubkey,   // Creator's quote ATA
    pub y0: u64,                     // Locked threshold
    pub investor_fee_share_bps: u16, // Max investor share
    pub daily_cap_quote: u64,        // Daily cap
    pub min_payout_lamports: u64,    // Minimum payout
    pub bump: u8,                    // PDA bump
    pub last_day_close_ts: i64,      // Last close timestamp
    pub status: u8,                  // Status flags
}
```

**Seeds:** `["policy", damm_pool]`  
**Space:** 464 bytes

### HonoraryPosition

```rust
pub struct HonoraryPosition {
    pub policy: Pubkey,  // Associated policy
    pub bump: u8,        // PDA bump
}
```

**Seeds:** `["honorary", policy]`  
**Space:** 40 bytes

### DistributionProgress

```rust
pub struct DistributionProgress {
    pub policy: Pubkey,              // Associated policy
    pub day_start_ts: i64,           // Current day start time
    pub page_cursor: u32,            // Pagination cursor
    pub claimed_quote: u64,          // Total claimed this day
    pub investor_distributed: u64,   // Total to investors this day
    pub carry_quote: u64,            // Dust carry-forward
    pub day_open: bool,              // Day open flag
}
```

**Seeds:** `["progress", damm_pool]`  
**Space:** 64 bytes

---

## Events

### HonoraryPositionInitialized

```rust
pub struct HonoraryPositionInitialized {
    pub policy: Pubkey,
    pub position: Pubkey,
    pub quote_treasury: Pubkey,
}
```

Emitted when honorary position is configured.

### QuoteFeesClaimed

```rust
pub struct QuoteFeesClaimed {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub quote_fees_claimed: u64,
    pub cumulative_claimed: u64,
    pub eligible_share_bps: u16,
}
```

Emitted after claiming fees from CP-AMM.

### InvestorPayoutPage

```rust
pub struct InvestorPayoutPage {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub page_start: u32,
    pub investors_processed: u32,
    pub total_paid_quote: u64,
    pub carry_quote: u64,
}
```

Emitted after processing each page of investors.

### CreatorPayoutDayClosed

```rust
pub struct CreatorPayoutDayClosed {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub creator_quote_paid: u64,
    pub investor_quote_paid: u64,
    pub claimed_quote: u64,
    pub share_bps: u16,
}
```

Emitted when day closes and remainder goes to creator.

---

## Error Codes

The program defines 79 error codes. Key errors:

| Code | Error | Description |
|------|-------|-------------|
| 6000 | `InvalidPoolAccount` | Failed to deserialize pool |
| 6001 | `InvalidPositionAccount` | Failed to deserialize position |
| 6002 | `UnsupportedPartnerPool` | Pool has non-default partner |
| 6003 | `BaseMintMismatch` | Base mint doesn't match pool |
| 6004 | `VaultMismatch` | Vault doesn't match pool |
| 6017 | `BaseFeeDetected` | Base fees detected (must be 0) |
| 6018 | `DayNotReady` | < 24h since last close |
| 6019 | `UnexpectedPageCursor` | Cursor mismatch |
| 6020 | `InvalidInvestorShare` | Share > 10000 BPS |

See `src/errors.rs` for complete list.

---

## Constants

```rust
pub const DAY_SECONDS: i64 = 86_400;      // 24 hours
pub const MAX_BASIS_POINTS: u16 = 10_000; // 100%
```

**PDA Seeds:**
```rust
pub const POLICY_SEED: &[u8] = b"policy";
pub const PROGRESS_SEED: &[u8] = b"progress";
pub const HONORARY_POSITION_SEED: &[u8] = b"honorary";
```

