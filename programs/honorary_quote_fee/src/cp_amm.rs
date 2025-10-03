use anchor_lang::{
    prelude::*,
    solana_program::{
        account_info::AccountInfo,
        instruction::{AccountMeta, Instruction},
        program::invoke_signed,
    },
};
use anchor_spl::token::{Mint, TokenAccount};

use carbon_meteora_damm_v2_decoder::accounts::pool::Pool as DammPoolAccount;

use crate::{
    errors::HonoraryQuoteFeeError,
    state::{HonoraryPosition, HONORARY_POSITION_SEED},
};

#[allow(dead_code)]
pub enum CollectFeeMode {
    Both,
    OnlyBase,
    OnlyQuote,
}

impl CollectFeeMode {
    pub fn as_u8(&self) -> u8 {
        match self {
            CollectFeeMode::Both => 0,
            CollectFeeMode::OnlyBase => 1,
            CollectFeeMode::OnlyQuote => 2,
        }
    }
}

const CLAIM_POSITION_FEE_DISCRIMINATOR: [u8; 8] = [0xd3, 0xa2, 0x21, 0x85, 0x11, 0x9a, 0x26, 0xb4];

pub fn assert_quote_only_pool(
    pool: &DammPoolAccount,
    expected_quote_mint: Pubkey,
    required_mode: CollectFeeMode,
) -> Result<()> {
    if pool.collect_fee_mode != required_mode.as_u8() {
        return err!(HonoraryQuoteFeeError::InvalidFeeMode);
    }

    let pool_quote_mint = Pubkey::new_from_array(pool.token_b_mint.to_bytes());
    require_keys_eq!(
        pool_quote_mint,
        expected_quote_mint,
        HonoraryQuoteFeeError::QuoteMintMismatch
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn invoke_claim_position_fee<'info>(
    policy_key: Pubkey,
    honorary_position: &Account<'info, HonoraryPosition>,
    cp_amm_program: &AccountInfo<'info>,
    pool: &AccountInfo<'info>,
    pool_authority: &AccountInfo<'info>,
    position: &AccountInfo<'info>,
    token_a_account: &Account<'info, TokenAccount>,
    token_b_account: &Account<'info, TokenAccount>,
    token_a_vault: &Account<'info, TokenAccount>,
    token_b_vault: &Account<'info, TokenAccount>,
    token_a_mint: &Account<'info, Mint>,
    token_b_mint: &Account<'info, Mint>,
    position_nft_account: &Account<'info, TokenAccount>,
    honorary_position_info: &AccountInfo<'info>,
    token_program_a: &AccountInfo<'info>,
    token_program_b: &AccountInfo<'info>,
    event_authority: &AccountInfo<'info>,
) -> Result<()> {
    let accounts = vec![
        AccountMeta::new_readonly(*pool_authority.key, false),
        AccountMeta::new(*pool.key, false),
        AccountMeta::new(*position.key, false),
        AccountMeta::new(token_a_account.key(), false),
        AccountMeta::new(token_b_account.key(), false),
        AccountMeta::new(token_a_vault.key(), false),
        AccountMeta::new(token_b_vault.key(), false),
        AccountMeta::new_readonly(token_a_mint.key(), false),
        AccountMeta::new_readonly(token_b_mint.key(), false),
        AccountMeta::new(position_nft_account.key(), false),
        AccountMeta::new(honorary_position.key(), true),
        AccountMeta::new_readonly(*token_program_a.key, false),
        AccountMeta::new_readonly(*token_program_b.key, false),
        AccountMeta::new_readonly(*event_authority.key, false),
        AccountMeta::new_readonly(*cp_amm_program.key, false),
    ];

    let ix = Instruction {
        program_id: *cp_amm_program.key,
        accounts,
        data: CLAIM_POSITION_FEE_DISCRIMINATOR.to_vec(),
    };

    let bump_seed = [honorary_position.bump];
    let seeds: [&[u8]; 3] = [HONORARY_POSITION_SEED, policy_key.as_ref(), &bump_seed];
    let signer_seeds: &[&[&[u8]]] = &[&seeds];

    let account_infos = vec![
        pool_authority.clone(),
        pool.clone(),
        position.clone(),
        token_a_account.to_account_info(),
        token_b_account.to_account_info(),
        token_a_vault.to_account_info(),
        token_b_vault.to_account_info(),
        token_a_mint.to_account_info(),
        token_b_mint.to_account_info(),
        position_nft_account.to_account_info(),
        honorary_position_info.clone(),
        token_program_a.clone(),
        token_program_b.clone(),
        event_authority.clone(),
        cp_amm_program.clone(),
    ];

    invoke_signed(&ix, &account_infos, signer_seeds)?;

    Ok(())
}
