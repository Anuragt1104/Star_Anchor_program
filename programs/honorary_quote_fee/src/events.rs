use anchor_lang::prelude::*;

#[event]
pub struct HonoraryPositionInitialized {
    pub policy: Pubkey,
    pub position: Pubkey,
    pub quote_treasury: Pubkey,
}

#[event]
pub struct QuoteFeesClaimed {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub quote_fees_claimed: u64,
    pub cumulative_claimed: u64,
    pub eligible_share_bps: u16,
}

#[event]
pub struct InvestorPayoutPage {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub page_start: u32,
    pub investors_processed: u32,
    pub total_paid_quote: u64,
    pub carry_quote: u64,
}

#[event]
pub struct CreatorPayoutDayClosed {
    pub policy: Pubkey,
    pub day_start_ts: i64,
    pub creator_quote_paid: u64,
    pub investor_quote_paid: u64,
    pub claimed_quote: u64,
    pub share_bps: u16,
}
