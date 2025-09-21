use anchor_lang::prelude::*;
use crate::state::{Policy, HonoraryPosition};

declare_id!("11111111111111111111111111111111");

#[program]
pub mod honorary_quote_fee {
    use super::*;

    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        params: InitializePolicyParams,
    ) -> Result<()> {
        let policy = &mut ctx.accounts.policy;
        
        // Initialize policy account
        policy.authority = ctx.accounts.authority.key();
        policy.pool = params.pool;
        policy.quote_mint = params.quote_mint;
        policy.created_at = Clock::get()?.unix_timestamp;
        policy.bump = ctx.bumps.policy;

        // TODO: Validate that the pool exists and is a valid DAMM v2 pool
        // TODO: Validate that the pool's collect_fee_mode is OnlyB
        // TODO: Validate that the quote_mint matches token B in the pool

        Ok(())
    }

    /// Begin a new 24h distribution cycle.
    pub fn start_day(
        ctx: Context<StartDay>,
        day_start_ts: i64,
    ) -> Result<()> {
        let progress = &mut ctx.accounts.daily_progress;

        progress.authority = ctx.accounts.authority.key();
        progress.policy = ctx.accounts.policy.key();
        progress.day_start_ts = day_start_ts;
        progress.closed = false;
        progress.bump = ctx.bumps.daily_progress;

        Ok(())
    }

    /// Close the current 24 h cycle – no further payouts can occur.
    pub fn close_day(ctx: Context<CloseDay>) -> Result<()> {
        let progress = &mut ctx.accounts.daily_progress;
        progress.closed = true;
        Ok(())
    }

    /// Create the PDA that will own an honorary, quote-only fee position for this policy.
    pub fn create_honorary_position(
        ctx: Context<CreateHonoraryPosition>,
    ) -> Result<()> {
        let pos = &mut ctx.accounts.honorary_position;

        pos.policy = ctx.accounts.policy.key();
        pos.pool = ctx.accounts.policy.pool;
        pos.quote_mint = ctx.accounts.policy.quote_mint;
        pos.bump = ctx.bumps.honorary_position;

        // in future: CPI to cp-amm to actually create / register the position
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializePolicyParams {
    /// The DAMM v2 pool this policy is for
    pub pool: Pubkey,
    
    /// The quote mint (must be token B in the pool)
    pub quote_mint: Pubkey,
}

#[derive(Accounts)]
pub struct InitializePolicy<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + core::mem::size_of::<Policy>(),
        seeds = [
            b"policy",
            pool.key().as_ref(),
        ],
        bump,
    )]
    pub policy: Account<'info, Policy>,

    /// The DAMM v2 pool this policy is for
    /// CHECK: Validated in handler
    pub pool: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartDay<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(has_one = authority)]
    pub policy: Account<'info, Policy>,

    #[account(
        init,
        payer = authority,
        space = 8 + core::mem::size_of::<state::DailyProgress>(),
        seeds = [
            b"day",
            policy.key().as_ref(),
            &day_start_ts.to_le_bytes(),
        ],
        bump,
    )]
    pub daily_progress: Account<'info, state::DailyProgress>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CloseDay<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(has_one = authority)]
    pub policy: Account<'info, Policy>,

    #[account(
        mut,
        seeds = [
            b"day",
            policy.key().as_ref(),
            &daily_progress.day_start_ts.to_le_bytes(),
        ],
        bump = daily_progress.bump,
    )]
    pub daily_progress: Account<'info, state::DailyProgress>,
}

#[derive(Accounts)]
pub struct CreateHonoraryPosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(has_one = authority)]
    pub policy: Account<'info, Policy>,

    #[account(
        init,
        payer = authority,
        space = 8 + core::mem::size_of::<state::HonoraryPosition>(),
        seeds = [
            b"honorary",
            policy.key().as_ref(),
        ],
        bump,
    )]
    pub honorary_position: Account<'info, state::HonoraryPosition>,

    pub system_program: Program<'info, System>,
}

pub mod state {
    use super::*;

    /// Per-policy metadata.

    #[account]
    pub struct Policy {
        /// The authority who can manage this policy
        pub authority: Pubkey,
        
        /// The DAMM v2 pool this policy is for
        pub pool: Pubkey,
        
        /// The quote mint (must be token B in the pool)
        pub quote_mint: Pubkey,
        
        /// Timestamp when the policy was created
        pub created_at: i64,
        
        /// Bump seed for the PDA
        pub bump: u8,
    }

    /// Tracks a single 24 h crank window.
    #[account]
    pub struct DailyProgress {
        /// Authority that opened the day (must match Policy authority)
        pub authority: Pubkey,
        /// Policy this day belongs to
        pub policy: Pubkey,
        /// Unix timestamp that marks the beginning of the day
        pub day_start_ts: i64,
        /// Whether the day has been closed
        pub closed: bool,
        /// PDA bump
        pub bump: u8,
    }

    /// PDA that will own/represent the quote-only fee position for a policy.
    #[account]
    pub struct HonoraryPosition {
        pub policy: Pubkey,
        pub pool: Pubkey,
        pub quote_mint: Pubkey,
        pub bump: u8,
    }

    /// Per-day aggregates captured after ingestion; placeholder for future logic.
    #[account]
    pub struct DistributionLedger {
        pub policy: Pubkey,
        pub day_start_ts: i64,
        pub total_quote_fees: u64,
        pub locked_sum: u128,
        pub bump: u8,
    }
}

pub mod errors {
    use anchor_lang::prelude::*;

    #[error_code]
    pub enum HonoraryQuoteFeeError {
        #[msg("Invalid pool: not a valid DAMM v2 pool")]
        InvalidPool,
        
        #[msg("Invalid fee mode: pool must be in quote-only mode (OnlyB)")]
        InvalidFeeMode,
        
        #[msg("Invalid quote mint: must match token B in the pool")]
        InvalidQuoteMint,
    }
}
