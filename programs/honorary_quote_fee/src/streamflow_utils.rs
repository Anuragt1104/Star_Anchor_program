use anchor_lang::prelude::*;
use anchor_lang::solana_program::account_info::AccountInfo;
use anchor_spl::token::TokenAccount;
use streamflow_sdk::state::Contract;

use crate::errors::HonoraryQuoteFeeError;

pub struct InvestorEntry {
    pub locked_amount: u64,
    pub token_account_index: usize,
}

#[inline(never)]
pub fn collect_investors<'info>(
    now: u64,
    accounts: &'info [AccountInfo<'info>],
    expected_quote_mint: Pubkey,
    _policy_pool: Pubkey,
) -> Result<Vec<InvestorEntry>> {
    require!(
        accounts.len() % 2 == 0,
        HonoraryQuoteFeeError::InvalidInvestorAccount
    );
    let half = accounts
        .len()
        .checked_div(2)
        .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
    let mut investors = Vec::with_capacity(half);

    for (chunk_idx, chunk) in accounts.chunks(2).enumerate() {
        let stream_account = &chunk[0];
        let investor_token_account_info = &chunk[1];

        require_keys_eq!(
            *stream_account.owner,
            streamflow_sdk::id(),
            HonoraryQuoteFeeError::InvalidInvestorAccount
        );

        let contract = load_stream_contract(stream_account)?;
        require_keys_eq!(
            Pubkey::new_from_array(contract.mint.to_bytes()),
            expected_quote_mint,
            HonoraryQuoteFeeError::StreamflowMintMismatch
        );

        let locked = locked_amount(&contract, now)?;

        let token_account: Account<TokenAccount> = Account::try_from(investor_token_account_info)?;
        require_keys_eq!(
            token_account.mint,
            expected_quote_mint,
            HonoraryQuoteFeeError::InvestorAtaMintMismatch
        );

        let expected_recipient = Pubkey::new_from_array(contract.recipient.to_bytes());
        require_keys_eq!(
            token_account.owner,
            expected_recipient,
            HonoraryQuoteFeeError::InvestorAtaOwnerMismatch
        );

        let expected_recipient_tokens =
            Pubkey::new_from_array(contract.recipient_tokens.to_bytes());
        require_keys_eq!(
            token_account.key(),
            expected_recipient_tokens,
            HonoraryQuoteFeeError::InvestorAtaOwnerMismatch
        );

        let index_mul = (chunk_idx as u64)
            .checked_mul(2)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?
            .checked_add(1)
            .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
        let idx_usize: usize = usize::try_from(index_mul)
            .map_err(|_| error!(HonoraryQuoteFeeError::ArithmeticOverflow))?;
        investors.push(InvestorEntry {
            locked_amount: locked,
            token_account_index: idx_usize,
        });
    }

    Ok(investors)
}

#[inline(never)]
pub fn load_stream_contract(account_info: &AccountInfo<'_>) -> Result<Contract> {
    let data = account_info.try_borrow_data()?;
    Contract::try_from_slice(&data)
        .map_err(|_| error!(HonoraryQuoteFeeError::InvalidInvestorAccount))
}

#[inline(never)]
pub fn locked_amount(contract: &Contract, now: u64) -> Result<u64> {
    let unlocked_now = contract
        .amount_withdrawn
        .checked_add(contract.available_to_claim(now, 100.0))
        .ok_or(HonoraryQuoteFeeError::ArithmeticOverflow)?;
    let unlocked_clamped = unlocked_now.min(contract.ix.net_amount_deposited);
    Ok(contract
        .ix
        .net_amount_deposited
        .saturating_sub(unlocked_clamped))
}

pub fn eligible_share_bps(locked_total: u128, y0: u64, max_share_bps: u16) -> u16 {
    if y0 == 0 || locked_total == 0 {
        return 0;
    }

    let ratio = locked_total
        .saturating_mul(10_000u128)
        .checked_div(y0 as u128)
        .unwrap_or(0);
    ratio.min(max_share_bps as u128) as u16
}
