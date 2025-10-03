use anchor_lang::prelude::*;

pub const POLICY_SEED: &[u8] = b"policy";
pub const HONORARY_POSITION_SEED: &[u8] = b"honorary";
pub const PROGRESS_SEED: &[u8] = b"progress";

pub struct PolicyStatus;
impl PolicyStatus {
    pub const HONORARY_READY: u8 = 1u8;
}

#[account]
#[derive(InitSpace)]
#[repr(C)]
pub struct Policy {
    pub authority: Pubkey,
    pub pool: Pubkey,
    pub pool_authority: Pubkey,
    pub cp_amm_program: Pubkey,
    pub quote_mint: Pubkey,
    pub base_mint: Pubkey,
    pub quote_vault: Pubkey,
    pub base_vault: Pubkey,
    pub position: Pubkey,
    pub position_nft_mint: Pubkey,
    pub position_nft_account: Pubkey,
    pub quote_treasury: Pubkey,
    pub base_fee_check: Pubkey,
    pub creator_quote_ata: Pubkey,
    pub y0: u64,
    pub daily_cap_quote: u64,
    pub min_payout_lamports: u64,
    pub last_day_close_ts: i64,
    pub investor_fee_share_bps: u16,
    pub bump: u8,
    pub status: u8,
}

impl Policy {
    pub const LEN: usize = 8 + core::mem::size_of::<Self>();
}

#[account]
#[derive(InitSpace)]
#[repr(C)]
pub struct HonoraryPosition {
    pub policy: Pubkey,
    pub bump: u8,
}

impl HonoraryPosition {
    pub const LEN: usize = 8 + core::mem::size_of::<Self>();
}

#[account]
#[derive(InitSpace)]
#[repr(C)]
pub struct DistributionProgress {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub page_cursor: u32,
    pub claimed_quote: u64,
    pub investor_distributed: u64,
    pub carry_quote: u64,
    pub day_open: bool,
    pub bump: u8,
}

impl DistributionProgress {
    pub const LEN: usize = 8 + core::mem::size_of::<Self>();
}
