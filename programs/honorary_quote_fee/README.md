# Honorary Quote Fee Module

Anchor-compatible module that manages a quote-only fee position on DAMM v2 and provides a permissionless daily distribution crank targeting Streamflow-locked investors.

## Design Highlights
- **Quote-only enforcement**: Policy initialization validates the pool configuration and the crank aborts if any base fees appear.
- **Program-owned PDA**: The honorary position PDA controls the DAMM position NFT and both treasury ATAs (quote + base guard).
- **Streamflow-aware payouts**: Investor weights derive from live Streamflow contract data (`locked_i(t)`), with dust carried forward until it can be paid.
- **24h gating with resumable pagination**: First crank each day enforces a 24h cool-down, later pages reuse the active day state and are idempotent.
- **Creator remainder routing**: After the last page the module forwards the creator’s share plus any investor dust when investors are no longer eligible (no locked balance).

## Instruction Workflow
### 1. `initialize_policy`
Registers immutable pool configuration and policy parameters.

| Account | Type | Notes |
| --- | --- | --- |
| `payer` | Signer | Funds account creations |
| `authority` | Signer | Policy authority (required for future admin actions) |
| `policy` | PDA (`["policy", pool]`) | Stores configuration & PDA bumps |
| `progress` | PDA (`["progress", pool]`) | Tracks daily progress state |
| `damm_pool` | Unchecked | DAMM v2 pool account (validated for quote-only mode) |
| `pool_authority` | Unchecked | DAMM pool authority |
| `damm_program` | Unchecked | DAMM v2 program id |
| `quote_mint`/`base_mint` | Mint | Must match pool tokens B/A |
| `quote_vault`/`base_vault` | TokenAccount | Must match pool vaults |
| `creator_quote_ata` | TokenAccount | Creator destination (quote mint) |
| `system_program` | Program | |

Parameters: `investor_fee_share_bps`, `y0`, `daily_cap_quote`, `min_payout_lamports`.

### 2. `configure_honorary_position`
Creates the honorary PDA, links the pre-created DAMM position, and materialises the treasury ATAs.

| Account | Type | Notes |
| --- | --- | --- |
| `authority` | Signer | Must match `policy.authority` |
| `policy` | Account | Mutated to record DAMM position + treasuries |
| `honorary_position` | PDA (`["honorary", policy]`) | New PDA; seeds used as signer for CPI |
| `position` | Unchecked | DAMM position account (must be empty) |
| `position_nft_mint` | Mint | Must be 0 decimals |
| `position_nft_account` | TokenAccount | Holds the position NFT, owned by PDA |
| `quote_treasury` | ATA | Created for PDA / quote mint |
| `base_fee_check` | ATA | Created for PDA / base mint (guard only) |
| `token_program`, `associated_token_program`, `system_program`, `rent` | Programs | |

### 3. `crank_quote_fee_distribution`
Permissionless daily crank (one or more pages per day).

| Account | Type | Notes |
| --- | --- | --- |
| `policy` | Account | Mutated (stores last day start) |
| `honorary_position` | Account | PDA signer |
| `progress` | PDA (`["progress", policy.pool]`) | Day tracking |
| `quote_treasury` | TokenAccount | PDA-owned ATA for quote mint |
| `base_fee_check` | TokenAccount | Must remain untouched (base fees guard) |
| `creator_quote_ata` | TokenAccount | Creator payout destination |
| `pool`, `pool_authority`, `position` | Unchecked | DAMM CPI accounts |
| `position_nft_account` | TokenAccount | NFT custody (read) |
| `base_vault`, `quote_vault` | TokenAccount | Pool vaults |
| `base_mint`, `quote_mint` | Mint | Token programs validated |
| `event_authority`, `cp_amm_program`, `token_program_a`, `token_program_b` | Unchecked | DAMM CPI accounts |
| `token_program` | Program<Token> | Used for payouts |
| Remaining accounts | Pairs of `(streamflow stream, investor quote ATA)` |

Parameters:
- `expected_page_cursor`: the cursor the caller expects to resume from (enforces idempotency).
- `max_page_cursor`: optional cap (0 = unlimited) to guard against accidental over-iteration.
- `is_last_page`: mark the final page to close the day and route creator remainder.

Pagination is resumed via the stored `progress.page_cursor`. Re-running a failed page with the same cursor is safe.

## Streamflow + Distribution Rules
- `locked_i(t)` is computed on-chain via `available_to_claim` + withdrawal totals, ensuring compatibility with pausing/top-ups.
- `f_locked(t) = locked_total / Y0` determines the eligibility fraction.
- Investor share `= min(investor_fee_share_bps, floor(f_locked * 10000))`.
- Daily cap (if >0) clamps the aggregate investor quote paid per day.
- Per-investor dust below `min_payout_lamports` is deferred; leftovers accumulate in `progress.carry_quote` and roll into the next attempt.
- If no investors remain locked (`share_bps == 0`), the module forwards any accumulated carry to the creator on day close.

## Quote-only Safety Nets
1. Policy initialization fails unless the pool advertises quote-only fee collection and matching mint/vault layout.
2. The crank aborts if `base_fee_check` balance changes after claiming fees.
3. Treasury ATAs are PDA-owned and re-derived on every call; mismatches trigger errors.

## PDA Seeds
- `policy` – `hash("policy" || pool_pubkey)`
- `honorary_position` – `hash("honorary" || policy_pubkey)`
- `progress` – `hash("progress" || pool_pubkey)`

## Events
- `HonoraryPositionInitialized { policy, position, quote_treasury }`
- `QuoteFeesClaimed { policy, day_start_ts, quote_fees_claimed, cumulative_claimed, eligible_share_bps }`
- `InvestorPayoutPage { policy, day_start_ts, page_start, investors_processed, total_paid_quote, carry_quote }`
- `CreatorPayoutDayClosed { policy, day_start_ts, creator_quote_paid, investor_quote_paid, claimed_quote, share_bps }`

## Error Codes (excerpt)
- `InvalidInvestorShare`, `InvalidY0`
- `InvalidPoolAccount`, `InvalidFeeMode`, `QuoteMintMismatch`, `BaseMintMismatch`, `VaultMismatch`
- `Unauthorized`, `HonoraryPositionAlreadyConfigured`, `HonoraryPositionNotReady`
- `PositionPoolMismatch`, `PositionHasUnclaimedFees`, `PositionNotEmpty`
- `BaseFeeDetected`, `UnexpectedPageCursor`, `PageOverflow`, `EmptyPageWithoutLastFlag`
- `InvestorAtaOwnerMismatch`, `InvestorAtaMintMismatch`, `StreamflowMintMismatch`

See `errors.rs` for full list.

## Testing & Verification Checklist
The local validator / bankrun suite is not bundled yet. Recommended scenarios before deployment:
1. **Happy path** – accrue quote fees, execute multi-page crank, verify investor/creator balances (including dust carry).
2. **All unlocked** – when every Streamflow contract is fully unlocked, ensure the crank routes 100% (plus historic carry) to the creator and resets the day.
3. **Daily cap** – configure a small `daily_cap_quote`, ensure payouts clamp and carry rolls to future days.
4. **Dust behaviour** – set `min_payout_lamports` above small payouts, verify dust defers and eventually settles once enough accumulates.
5. **Base-fee guard** – intentionally misconfigure the DAMM pool or manually seed base fees; crank should fail with `BaseFeeDetected` and perform no transfers.
6. **Pagination replay** – interrupt a page mid-run and re-submit with the same cursor; distribution must remain consistent and idempotent.
7. **Quote-only validation** – attempt to initialize a policy with mismatched mint/vaults; expect deterministic failure.

### Build status
`cargo fmt` has been applied. `cargo check -p honorary_quote_fee` currently times out during the first dependency build on this environment—rerun locally after the initial crate fetch to confirm.

## Integration Notes
- The DAMM position must exist and remain empty prior to `configure_honorary_position`; creation CPI wiring can be added upstream if desired.
- Pass Streamflow stream accounts and investor ATAs as `[stream, ata, stream, ata, ...]` in each crank invocation.
- Use `max_page_cursor` to protect against unbounded pagination if orchestrating via off-chain automation.
- The creator ATA must remain writable; distribution to investors should tolerate self-managed ATAs (create them on demand off-chain if missing).

## TODOs / Follow-ups
- Add an integration test harness (local validator or bankrun) covering the scenarios listed above.
- Consider exposing an optional admin hook to update policy parameters (e.g., caps) if governance requires.
- Wire cp-amm position creation CPI if automatic provisioning is preferred.
- Assess gas budgeting once real investor batch sizes are known (current math is `O(n)` over the supplied page).

