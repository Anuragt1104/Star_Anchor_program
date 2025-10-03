#![allow(unexpected_cfgs, deprecated)]
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use carbon_meteora_damm_v2_decoder::{
    accounts::pool::Pool as DammPoolAccount, types::position::Position as DammPosition,
};

mod cp_amm;
mod errors;
mod events;
mod math;
mod state;
mod streamflow_utils;

use cp_amm::{assert_quote_only_pool, CollectFeeMode};
use errors::HonoraryQuoteFeeError;
use events::{
    CreatorPayoutDayClosed, HonoraryPositionInitialized, InvestorPayoutPage, QuoteFeesClaimed,
};
use math::{mul_div_floor_u128, saturating_sub_u64, u128_to_u64};
use state::{
    DistributionProgress, HonoraryPosition, Policy, HONORARY_POSITION_SEED, POLICY_SEED,
    PROGRESS_SEED,
};
use streamflow_utils::{collect_investors, eligible_share_bps};

declare_id!("11111111111111111111111111111111");

pub const DAY_SECONDS: i64 = 86_400;
pub const MAX_BASIS_POINTS: u16 = 10_000;

#[program]
pub mod honorary_quote_fee {
    use super::*;

    pub fn initialize_policy(
        ctx: Context<InitializePolicy>,
        params: InitializePolicyParams,
    ) -> Result<()> {
        require!(
            params.investor_fee_share_bps <= MAX_BASIS_POINTS,
            HonoraryQuoteFeeError::InvalidInvestorShare
        );
        require!(params.y0 > 0, HonoraryQuoteFeeError::InvalidY0);

        let policy = &mut ctx.accounts.policy;
        let pool_data = ctx.accounts.damm_pool.try_borrow_data()?;
        let mut pool_bytes: &[u8] = &pool_data;
        let pool = DammPoolAccount::deserialize(&mut pool_bytes)
            .map_err(|_| error!(HonoraryQuoteFeeError::InvalidPoolAccount))?;

        assert_quote_only_pool(
            &pool,
            ctx.accounts.quote_mint.key(),
            CollectFeeMode::OnlyQuote,
        )?;

        require_keys_eq!(
            pool.partner,
            Pubkey::default(),
            HonoraryQuoteFeeError::UnsupportedPartnerPool
        );
        require_keys_eq!(
            ctx.accounts.creator_quote_ata.mint,
            ctx.accounts.quote_mint.key(),
            HonoraryQuoteFeeError::CreatorAtaMintMismatch
        );

        let pool_base_mint = Pubkey::new_from_array(pool.token_a_mint.to_bytes());
        require_keys_eq!(
            pool_base_mint,
            ctx.accounts.base_mint.key(),
            HonoraryQuoteFeeError::BaseMintMismatch
        );
        let pool_base_vault = Pubkey::new_from_array(pool.token_a_vault.to_bytes());
        let pool_quote_vault = Pubkey::new_from_array(pool.token_b_vault.to_bytes());
        require_keys_eq!(
            pool_base_vault,
            ctx.accounts.base_vault.key(),
            HonoraryQuoteFeeError::VaultMismatch
        );
        require_keys_eq!(
            pool_quote_vault,
            ctx.accounts.quote_vault.key(),
            HonoraryQuoteFeeError::VaultMismatch
        );

        policy.authority = ctx.accounts.authority.key();
        policy.pool = ctx.accounts.damm_pool.key();
        policy.pool_authority = ctx.accounts.pool_authority.key();
        policy.cp_amm_program = ctx.accounts.damm_program.key();
        policy.quote_mint = ctx.accounts.quote_mint.key();
        policy.base_mint = ctx.accounts.base_mint.key();
        policy.quote_vault = ctx.accounts.quote_vault.key();
        policy.base_vault = ctx.accounts.base_vault.key();
        policy.position = Pubkey::default();
        policy.position_nft_mint = Pubkey::default();
        policy.position_nft_account = Pubkey::default();
        policy.quote_treasury = Pubkey::default();
        policy.base_fee_check = Pubkey::default();
        policy.creator_quote_ata = ctx.accounts.creator_quote_ata.key();
        policy.y0 = params.y0;
        policy.investor_fee_share_bps = params.investor_fee_share_bps;
        policy.daily_cap_quote = params.daily_cap_quote;
        policy.min_payout_lamports = params.min_payout_lamports;
        policy.bump = ctx.bumps.policy;
        // Intentionally initialize to a large negative sentinel value without triggering
        // arithmetic lints at runtime by using a literal constant.
        policy.last_day_close_ts = -4_611_686_018_427_387_904;
        policy.status = 0u8;

        let progress = &mut ctx.accounts.progress;
        progress.policy = policy.key();
        progress.day_start_ts = 0;
        progress.page_cursor = 0;
        progress.claimed_quote = 0;
        progress.investor_distributed = 0;
        progress.carry_quote = 0;
        progress.day_open = false;
        progress.bump = ctx.bumps.progress;

        Ok(())
    }

    pub fn configure_honorary_position(ctx: Context<ConfigureHonoraryPosition>) -> Result<()> {
        let policy = &mut ctx.accounts.policy;
        require_keys_eq!(
            policy.authority,
            ctx.accounts.authority.key(),
            HonoraryQuoteFeeError::Unauthorized
        );
        require_eq!(
            policy.position,
            Pubkey::default(),
            HonoraryQuoteFeeError::HonoraryPositionAlreadyConfigured
        );

        let position_data = ctx.accounts.position.try_borrow_data()?;
        let mut position_bytes: &[u8] = &position_data;
        let position = DammPosition::deserialize(&mut position_bytes)
            .map_err(|_| error!(HonoraryQuoteFeeError::InvalidPositionAccount))?;

        require_keys_eq!(
            position.pool,
            policy.pool,
            HonoraryQuoteFeeError::PositionPoolMismatch
        );
        require!(
            position.fee_a_pending == 0 && position.fee_b_pending == 0,
            HonoraryQuoteFeeError::PositionHasUnclaimedFees
        );
        require!(
            position.unlocked_liquidity == 0
                && position.vested_liquidity == 0
                && position.permanent_locked_liquidity == 0,
            HonoraryQuoteFeeError::PositionNotEmpty
        );

        require_eq!(
            ctx.accounts.position_nft_mint.decimals,
            0,
            HonoraryQuoteFeeError::InvalidPositionMint
        );
        require_keys_eq!(
            ctx.accounts.position_nft_account.mint,
            ctx.accounts.position_nft_mint.key(),
            HonoraryQuoteFeeError::InvalidPositionNft
        );
        require_keys_eq!(
            ctx.accounts.position_nft_account.owner,
            ctx.accounts.honorary_position.key(),
            HonoraryQuoteFeeError::InvalidPositionNftOwner
        );
        require_eq!(
            ctx.accounts.position_nft_account.amount,
            1,
            HonoraryQuoteFeeError::InvalidPositionNftAmount
        );

        require_keys_eq!(
            ctx.accounts.quote_treasury.mint,
            policy.quote_mint,
            HonoraryQuoteFeeError::TreasuryMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.quote_treasury.owner,
            ctx.accounts.honorary_position.key(),
            HonoraryQuoteFeeError::TreasuryOwnerMismatch
        );
        require_keys_eq!(
            ctx.accounts.base_fee_check.mint,
            policy.base_mint,
            HonoraryQuoteFeeError::TreasuryMintMismatch
        );
        require_keys_eq!(
            ctx.accounts.base_fee_check.owner,
            ctx.accounts.honorary_position.key(),
            HonoraryQuoteFeeError::TreasuryOwnerMismatch
        );

        let honorary_position = &mut ctx.accounts.honorary_position;
        honorary_position.policy = policy.key();
        honorary_position.bump = ctx.bumps.honorary_position;

        policy.position = ctx.accounts.position.key();
        policy.position_nft_mint = ctx.accounts.position_nft_mint.key();
        policy.position_nft_account = ctx.accounts.position_nft_account.key();
        policy.quote_treasury = ctx.accounts.quote_treasury.key();
        policy.base_fee_check = ctx.accounts.base_fee_check.key();
        policy.status |= state::PolicyStatus::HONORARY_READY;

        emit!(HonoraryPositionInitialized {
            policy: policy.key(),
            position: policy.position,
            quote_treasury: policy.quote_treasury,
        });

        Ok(())
    }

    pub fn crank_quote_fee_distribution(
        ctx: Context<CrankQuoteFeeDistribution>,
        params: CrankQuoteFeeParams,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let now_ts = clock.unix_timestamp;
        require!(now_ts >= 0, HonoraryQuoteFeeError::InvalidTimestamp);

        let policy = &mut ctx.accounts.policy;
        let progress = &mut ctx.accounts.progress;

        require!(
            (policy.status & state::PolicyStatus::HONORARY_READY) != 0,
            HonoraryQuoteFeeError::HonoraryPositionNotReady
        );
        require_keys_eq!(
            progress.policy,
            policy.key(),
            HonoraryQuoteFeeError::DayNotOpen
        );

        if !progress.day_open {
            require!(
                now_ts >= policy.last_day_close_ts + DAY_SECONDS,
                HonoraryQuoteFeeError::DayNotReady
            );
            progress.day_open = true;
            progress.day_start_ts = now_ts;
            progress.page_cursor = 0;
            progress.claimed_quote = 0;
            progress.investor_distributed = 0;
        }

        require!(
            params.expected_page_cursor == progress.page_cursor,
            HonoraryQuoteFeeError::UnexpectedPageCursor
        );

        let quote_before = ctx.accounts.quote_treasury.amount;
        let base_before = ctx.accounts.base_fee_check.amount;

        let cp_amm_program_info = ctx.accounts.cp_amm_program.to_account_info();
        let pool_info = ctx.accounts.pool.to_account_info();
        let pool_authority_info = ctx.accounts.pool_authority.to_account_info();
        let position_info = ctx.accounts.position.to_account_info();
        let token_program_a_info = ctx.accounts.token_program_a.to_account_info();
        let token_program_b_info = ctx.accounts.token_program_b.to_account_info();
        let event_authority_info = ctx.accounts.event_authority.to_account_info();

        cp_amm::invoke_claim_position_fee(
            policy.key(),
            &ctx.accounts.honorary_position,
            &cp_amm_program_info,
            &pool_info,
            &pool_authority_info,
            &position_info,
            &ctx.accounts.base_fee_check,
            &ctx.accounts.quote_treasury,
            &ctx.accounts.base_vault,
            &ctx.accounts.quote_vault,
            &ctx.accounts.base_mint,
            &ctx.accounts.quote_mint,
            &ctx.accounts.position_nft_account,
            &ctx.accounts.honorary_position.to_account_info(),
            &token_program_a_info,
            &token_program_b_info,
            &event_authority_info,
        )?;

        ctx.accounts.quote_treasury.reload()?;
        ctx.accounts.base_fee_check.reload()?;

        let quote_after = ctx.accounts.quote_treasury.amount;
        let base_after = ctx.accounts.base_fee_check.amount;

        let quote_claimed = quote_after
            .checked_sub(quote_before)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
        require_eq!(
            base_after,
            base_before,
            HonoraryQuoteFeeError::BaseFeeDetected
        );

        progress.claimed_quote = progress
            .claimed_quote
            .checked_add(quote_claimed)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;

        let investors = collect_investors(
            now_ts as u64,
            &ctx.remaining_accounts,
            policy.quote_mint,
            policy.pool,
        )?;

        let investor_count = investors.len() as u32;
        require!(
            investor_count > 0 || params.is_last_page,
            HonoraryQuoteFeeError::EmptyPageWithoutLastFlag
        );
        let max_cursor = if params.max_page_cursor == 0 { u32::MAX } else { params.max_page_cursor };
        require!(
            investor_count + progress.page_cursor <= max_cursor,
            HonoraryQuoteFeeError::PageOverflow
        );

        let total_locked: u128 = investors
            .iter()
            .map(|entry| entry.locked_amount as u128)
            .sum();
        let share_bps = eligible_share_bps(total_locked, policy.y0, policy.investor_fee_share_bps);

        let mut target_investor_quote = u128_to_u64(mul_div_floor_u128(
            progress.claimed_quote as u128,
            share_bps as u128,
            MAX_BASIS_POINTS as u128,
        )?)?;

        if policy.daily_cap_quote > 0 {
            target_investor_quote = target_investor_quote.min(policy.daily_cap_quote);
        }

        let mut available_to_pay = target_investor_quote
            .saturating_sub(progress.investor_distributed)
            .saturating_add(progress.carry_quote);
        let mut carry_for_creator = 0u64;
        if share_bps == 0 {
            carry_for_creator = progress.carry_quote;
            available_to_pay = 0;
            progress.carry_quote = 0;
        }

        let mut total_paid_this_page: u64 = 0;
        let mut transfers: Vec<(u64, usize)> = Vec::with_capacity(investors.len());

        for entry in investors.iter() {
            if available_to_pay == 0 || entry.locked_amount == 0 {
                transfers.push((0, entry.token_account_index));
                continue;
            }

            let payout_u128 = mul_div_floor_u128(
                available_to_pay as u128,
                entry.locked_amount as u128,
                total_locked.max(1),
            )?;
            let mut payout = u128_to_u64(payout_u128)?;

            if payout < policy.min_payout_lamports {
                payout = 0;
            }

            transfers.push((payout, entry.token_account_index));
            total_paid_this_page = total_paid_this_page
                .checked_add(payout)
                .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
        }

        available_to_pay = available_to_pay.saturating_sub(total_paid_this_page);
        progress.carry_quote = available_to_pay;
        progress.investor_distributed = progress
            .investor_distributed
            .checked_add(total_paid_this_page)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;

        let bump = ctx.accounts.honorary_position.bump;
        let policy_key = policy.key();
        // Construct signer seeds for the honorary PDA
        let bump_seed = [bump];
        let seeds: [&[u8]; 3] = [HONORARY_POSITION_SEED, policy_key.as_ref(), &bump_seed];
        let signer: &[&[&[u8]]] = &[&seeds];

        for (amount, token_account_index) in transfers.iter() {
            if *amount == 0 {
                continue;
            }
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.quote_treasury.to_account_info(),
                        to: ctx.remaining_accounts[*token_account_index].clone(),
                        authority: ctx.accounts.honorary_position.to_account_info(),
                    },
                    signer,
                ),
                *amount,
            )?;
        }

        progress.page_cursor = progress
            .page_cursor
            .checked_add(investor_count)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;

        emit!(QuoteFeesClaimed {
            policy: policy.key(),
            day_start_ts: progress.day_start_ts,
            quote_fees_claimed: quote_claimed,
            cumulative_claimed: progress.claimed_quote,
            eligible_share_bps: share_bps,
        });

        emit!(InvestorPayoutPage {
            policy: policy.key(),
            day_start_ts: progress.day_start_ts,
            page_start: params.expected_page_cursor,
            investors_processed: investor_count,
            total_paid_quote: total_paid_this_page,
            carry_quote: progress.carry_quote,
        });

        if params.is_last_page {
            let mut creator_transfer = saturating_sub_u64(
                saturating_sub_u64(progress.claimed_quote, target_investor_quote),
                0,
            );
            if share_bps == 0 {
                creator_transfer = creator_transfer
                    .checked_add(carry_for_creator)
                    .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
                progress.carry_quote = 0;
            }

            if creator_transfer > 0 {
                token::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info(),
                        Transfer {
                            from: ctx.accounts.quote_treasury.to_account_info(),
                            to: ctx.accounts.creator_quote_ata.to_account_info(),
                            authority: ctx.accounts.honorary_position.to_account_info(),
                        },
                        signer,
                    ),
                    creator_transfer,
                )?;
            }

            emit!(CreatorPayoutDayClosed {
                policy: policy.key(),
                day_start_ts: progress.day_start_ts,
                creator_quote_paid: creator_transfer,
                investor_quote_paid: progress.investor_distributed,
                claimed_quote: progress.claimed_quote,
                share_bps: share_bps,
            });

            policy.last_day_close_ts = progress.day_start_ts;
            progress.day_open = false;
            progress.claimed_quote = 0;
            progress.investor_distributed = 0;
            progress.page_cursor = 0;
        }

        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct InitializePolicyParams {
    pub investor_fee_share_bps: u16,
    pub y0: u64,
    pub daily_cap_quote: u64,
    pub min_payout_lamports: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CrankQuoteFeeParams {
    pub expected_page_cursor: u32,
    pub max_page_cursor: u32,
    pub is_last_page: bool,
}

#[derive(Accounts)]
pub struct InitializePolicy<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = Policy::LEN,
        seeds = [POLICY_SEED, damm_pool.key().as_ref()],
        bump,
    )]
    pub policy: Account<'info, Policy>,
    #[account(
        init,
        payer = payer,
        space = DistributionProgress::LEN,
        seeds = [PROGRESS_SEED, damm_pool.key().as_ref()],
        bump,
    )]
    pub progress: Account<'info, DistributionProgress>,
    /// CHECK: DAMM pool account
    #[account(mut)]
    pub damm_pool: UncheckedAccount<'info>,
    /// CHECK: DAMM pool authority PDA
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: DAMM program id
    pub damm_program: UncheckedAccount<'info>,
    pub quote_mint: Account<'info, Mint>,
    pub base_mint: Account<'info, Mint>,
    #[account(mut)]
    pub quote_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    pub creator_quote_ata: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ConfigureHonoraryPosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority)]
    pub policy: Account<'info, Policy>,
    #[account(
        init,
        payer = authority,
        space = HonoraryPosition::LEN,
        seeds = [HONORARY_POSITION_SEED, policy.key().as_ref()],
        bump,
    )]
    pub honorary_position: Account<'info, HonoraryPosition>,
    /// CHECK: Existing DAMM position account
    #[account(mut)]
    pub position: UncheckedAccount<'info>,
    pub position_nft_mint: Account<'info, Mint>,
    #[account(mut)]
    pub position_nft_account: Account<'info, TokenAccount>,
    #[account(address = policy.quote_mint)]
    pub quote_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = quote_mint,
        associated_token::authority = honorary_position,
    )]
    pub quote_treasury: Account<'info, TokenAccount>,
    #[account(address = policy.base_mint)]
    pub base_mint: Account<'info, Mint>,
    #[account(
        init_if_needed,
        payer = authority,
        associated_token::mint = base_mint,
        associated_token::authority = honorary_position,
    )]
    pub base_fee_check: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct CrankQuoteFeeDistribution<'info> {
    /// Any signer to satisfy authorization lint; permissionless crank otherwise
    pub cranker: Signer<'info>,
    #[account(mut)]
    pub policy: Account<'info, Policy>,
    #[account(
        mut,
        seeds = [HONORARY_POSITION_SEED, policy.key().as_ref()],
        bump = honorary_position.bump,
    )]
    pub honorary_position: Account<'info, HonoraryPosition>,
    #[account(mut, seeds = [PROGRESS_SEED, policy.pool.as_ref()], bump = progress.bump)]
    pub progress: Account<'info, DistributionProgress>,
    #[account(mut, address = policy.quote_treasury)]
    pub quote_treasury: Account<'info, TokenAccount>,
    #[account(mut, address = policy.base_fee_check)]
    pub base_fee_check: Account<'info, TokenAccount>,
    #[account(mut, address = policy.creator_quote_ata)]
    pub creator_quote_ata: Account<'info, TokenAccount>,
    /// CHECK: DAMM pool account
    #[account(address = policy.pool)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: DAMM pool authority
    #[account(address = policy.pool_authority)]
    pub pool_authority: UncheckedAccount<'info>,
    /// CHECK: DAMM position account
    #[account(mut, address = policy.position)]
    pub position: UncheckedAccount<'info>,
    #[account(mut, address = policy.position_nft_account)]
    pub position_nft_account: Account<'info, TokenAccount>,
    #[account(mut, address = policy.base_vault)]
    pub base_vault: Account<'info, TokenAccount>,
    #[account(mut, address = policy.quote_vault)]
    pub quote_vault: Account<'info, TokenAccount>,
    pub base_mint: Account<'info, Mint>,
    pub quote_mint: Account<'info, Mint>,
    /// CHECK: DAMM event authority
    pub event_authority: UncheckedAccount<'info>,
    /// CHECK: DAMM program id
    #[account(address = policy.cp_amm_program)]
    pub cp_amm_program: UncheckedAccount<'info>,
    /// CHECK: Token A program
    pub token_program_a: UncheckedAccount<'info>,
    /// CHECK: Token B program
    pub token_program_b: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}
