use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
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
        args: StartDayParams,
    ) -> Result<()> {
        let progress = &mut ctx.accounts.daily_progress;

        progress.authority = ctx.accounts.authority.key();
        progress.policy = ctx.accounts.policy.key();
        progress.day_start_ts = args.day_start_ts;
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

    /// Distribute quote tokens from the honorary position's treasury to multiple recipients
    /// based on basis-point splits.
    pub fn distribute(
        ctx: Context<Distribute>,
        params: DistributeParams,
    ) -> Result<()> {
        // Validate splits
        if params.splits.is_empty() {
            return err!(errors::HonoraryQuoteFeeError::EmptySplits);
        }
        
        // Ensure sum of bps is exactly 10_000
        let total_bps: u16 = params.splits.iter().map(|s| s.bps).sum();
        if total_bps != 10_000 {
            return err!(errors::HonoraryQuoteFeeError::InvalidSplitsBps);
        }
        
        // Validate treasury
        if ctx.accounts.treasury.mint != ctx.accounts.policy.quote_mint {
            return err!(errors::HonoraryQuoteFeeError::TreasuryMismatch);
        }
        
        if ctx.accounts.treasury.owner != ctx.accounts.honorary_position.key() {
            return err!(errors::HonoraryQuoteFeeError::TreasuryMismatch);
        }
        
        // Check treasury has sufficient balance
        let to_distribute = params.amount;
        if ctx.accounts.treasury.amount < to_distribute {
            return err!(errors::HonoraryQuoteFeeError::InsufficientTreasuryBalance);
        }
        
        // Get remaining accounts as recipient token accounts
        let remaining_accounts = ctx.remaining_accounts;
        if remaining_accounts.len() != params.splits.len() {
            return err!(errors::HonoraryQuoteFeeError::RecipientAtaMismatch);
        }
        
        // Calculate and distribute amounts to recipients
        let mut remaining_amount = to_distribute;
        let honorary_position_key = ctx.accounts.honorary_position.key();
        let seeds = &[
            b"honorary", 
            ctx.accounts.policy.key().as_ref(),
            &[ctx.accounts.honorary_position.bump]
        ];
        let signer_seeds = &[&seeds[..]];
        
        for (i, split) in params.splits.iter().enumerate() {
            let recipient_account_info = &remaining_accounts[i];
            let recipient_token_account: Account<TokenAccount> = Account::try_from(recipient_account_info)?;
            
            // Validate recipient token account
            if recipient_token_account.mint != ctx.accounts.policy.quote_mint {
                return err!(errors::HonoraryQuoteFeeError::RecipientAtaMismatch);
            }
            
            if recipient_token_account.owner != split.recipient {
                return err!(errors::HonoraryQuoteFeeError::RecipientAtaMismatch);
            }
            
            // Calculate amount for this recipient
            let mut recipient_amount = if i == params.splits.len() - 1 {
                // Last recipient gets the remainder to handle rounding
                remaining_amount
            } else {
                (to_distribute as u128 * split.bps as u128 / 10_000u128) as u64
            };
            
            // Transfer tokens
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.treasury.to_account_info(),
                        to: recipient_account_info.clone(),
                        authority: ctx.accounts.honorary_position.to_account_info(),
                    },
                    signer_seeds,
                ),
                recipient_amount,
            )?;
            
            remaining_amount = remaining_amount.saturating_sub(recipient_amount);
        }
        
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

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SplitInput {
    /// Recipient public key (owner of the token account)
    pub recipient: Pubkey,
    
    /// Basis points (out of 10_000) for this recipient
    pub bps: u16,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct DistributeParams {
    /// Amount of quote tokens to distribute
    pub amount: u64,
    
    /// List of recipients and their basis point allocations
    pub splits: Vec<SplitInput>,
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
        init_if_needed,
        payer = authority,
        space = 8 + core::mem::size_of::<state::DailyProgress>(),
        seeds = [
            b"day",
            policy.key().as_ref(),
        ],
        bump,
    )]
    pub daily_progress: Account<'info, state::DailyProgress>,

    pub system_program: Program<'info, System>,
}

/// Parameters for starting a new 24 h cycle.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct StartDayParams {
    pub day_start_ts: i64,
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

#[derive(Accounts)]
pub struct Distribute<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(has_one = authority)]
    pub policy: Account<'info, Policy>,

    #[account(
        seeds = [
            b"honorary",
            policy.key().as_ref(),
        ],
        bump = honorary_position.bump,
    )]
    pub honorary_position: Account<'info, state::HonoraryPosition>,

    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
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

        #[msg("Empty splits: must provide at least one recipient")]
        EmptySplits,

        #[msg("Invalid splits: sum of basis points must be exactly 10,000")]
        InvalidSplitsBps,

        #[msg("Treasury mismatch: mint or owner is incorrect")]
        TreasuryMismatch,

        #[msg("Recipient ATA mismatch: mint or owner is incorrect")]
        RecipientAtaMismatch,

        #[msg("Insufficient treasury balance for distribution")]
        InsufficientTreasuryBalance,
    }
}
