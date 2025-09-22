use anchor_lang::prelude::*;

#[error_code]
pub enum HonoraryQuoteFeeError {
    #[msg("Investor share basis points must be <= 10,000")]
    InvalidInvestorShare,
    #[msg("Initial investor allocation Y0 must be greater than zero")]
    InvalidY0,
    #[msg("Invalid or unreadable DAMM pool account")]
    InvalidPoolAccount,
    #[msg("DAMM pool must be configured for quote-only fee collection")]
    InvalidFeeMode,
    #[msg("Pool quote mint does not match policy quote mint")]
    QuoteMintMismatch,
    #[msg("Pool base mint does not match policy base mint")]
    BaseMintMismatch,
    #[msg("Pool vault configuration mismatch")]
    VaultMismatch,
    #[msg("Unsupported partner configuration on DAMM pool")]
    UnsupportedPartnerPool,
    #[msg("Missing PDA bump in context")]
    MissingBump,
    #[msg("Unauthorized authority for this policy")]
    Unauthorized,
    #[msg("Invalid or unreadable DAMM position account")]
    InvalidPositionAccount,
    #[msg("DAMM position does not belong to the configured pool")]
    PositionPoolMismatch,
    #[msg("Honorary position must start with zero pending fees")]
    PositionHasUnclaimedFees,
    #[msg("Honorary position must start with zero liquidity")]
    PositionNotEmpty,
    #[msg("Position NFT account mint mismatch")]
    InvalidPositionNft,
    #[msg("Position NFT mint must have zero decimals")]
    InvalidPositionMint,
    #[msg("Position NFT account owner must be the honorary PDA")]
    InvalidPositionNftOwner,
    #[msg("Position NFT account must hold exactly one token")]
    InvalidPositionNftAmount,
    #[msg("Unix timestamp must be non-negative")]
    InvalidTimestamp,
    #[msg("24h distribution window not yet available")]
    DayNotReady,
    #[msg("Pagination cursor mismatch")]
    UnexpectedPageCursor,
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("Claim produced base-denominated fees unexpectedly")]
    BaseFeeDetected,
    #[msg("Pagination page contains no investors but not marked final")]
    EmptyPageWithoutLastFlag,
    #[msg("Pagination cursor would overflow configured bound")]
    PageOverflow,
    #[msg("Invalid investor inputs")]
    InvalidInvestorAccount,
    #[msg("Streamflow contract mint mismatch")]
    StreamflowMintMismatch,
    #[msg("Investor token account owner mismatch")]
    InvestorAtaOwnerMismatch,
    #[msg("Investor token account mint mismatch")]
    InvestorAtaMintMismatch,
    #[msg("Creator quote ATA mint mismatch")]
    CreatorAtaMintMismatch,
    #[msg("Day not initialized")]
    DayNotOpen,
    #[msg("Honorary position not yet configured for policy")]
    HonoraryPositionNotReady,
    #[msg("Honorary position already configured")]
    HonoraryPositionAlreadyConfigured,
    #[msg("Treasury account mint mismatch")]
    TreasuryMintMismatch,
    #[msg("Treasury account owner mismatch")]
    TreasuryOwnerMismatch,
}
