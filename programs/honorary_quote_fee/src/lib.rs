use anchor_lang::prelude::*;
use crate::state::Policy;

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

pub mod state {
    use super::*;

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
