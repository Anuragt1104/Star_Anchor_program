use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;

pub struct MeteoraDammV2Decoder;

pub mod types {
    use super::{BorshDeserialize, Pubkey};

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct BaseFeeStruct {
        pub cliff_fee_numerator: u64,
        pub fee_scheduler_mode: u8,
        pub padding_0: [u8; 5],
        pub number_of_period: u16,
        pub period_frequency: u64,
        pub reduction_factor: u64,
        pub padding_1: u64,
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct DynamicFeeStruct {
        pub initialized: u8,
        pub padding: [u8; 7],
        pub max_volatility_accumulator: u32,
        pub variable_fee_control: u32,
        pub bin_step: u16,
        pub filter_period: u16,
        pub decay_period: u16,
        pub reduction_factor: u16,
        pub last_update_timestamp: u64,
        pub bin_step_u128: u128,
        pub sqrt_price_reference: u128,
        pub volatility_accumulator: u128,
        pub volatility_reference: u128,
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct PoolFeesStruct {
        pub base_fee: BaseFeeStruct,
        pub protocol_fee_percent: u8,
        pub partner_fee_percent: u8,
        pub referral_fee_percent: u8,
        pub padding_0: [u8; 5],
        pub dynamic_fee: DynamicFeeStruct,
        pub padding_1: [u64; 2],
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct PoolMetrics {
        pub total_lp_a_fee: u128,
        pub total_lp_b_fee: u128,
        pub total_protocol_a_fee: u64,
        pub total_protocol_b_fee: u64,
        pub total_partner_a_fee: u64,
        pub total_partner_b_fee: u64,
        pub total_position: u64,
        pub padding: u64,
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct RewardInfo {
        pub initialized: u8,
        pub reward_token_flag: u8,
        pub padding_0: [u8; 6],
        pub padding_1: [u8; 8],
        pub mint: Pubkey,
        pub vault: Pubkey,
        pub funder: Pubkey,
        pub reward_duration: u64,
        pub reward_duration_end: u64,
        pub reward_rate: u128,
        pub reward_per_token_stored: [u8; 32],
        pub last_update_time: u64,
        pub cumulative_seconds_with_empty_liquidity_reward: u64,
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct PositionMetrics {
        pub total_claimed_a_fee: u64,
        pub total_claimed_b_fee: u64,
    }

    #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
    pub struct UserRewardInfo {
        pub reward_per_token_checkpoint: [u8; 32],
        pub reward_pendings: u64,
        pub total_claimed_rewards: u64,
    }

    pub mod position {
        use super::*;

        #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
        pub struct Position {
            pub pool: Pubkey,
            pub nft_mint: Pubkey,
            pub fee_a_per_token_checkpoint: [u8; 32],
            pub fee_b_per_token_checkpoint: [u8; 32],
            pub fee_a_pending: u64,
            pub fee_b_pending: u64,
            pub unlocked_liquidity: u128,
            pub vested_liquidity: u128,
            pub permanent_locked_liquidity: u128,
            pub metrics: super::PositionMetrics,
            pub reward_infos: [super::UserRewardInfo; 2],
            pub padding: [u128; 6],
        }

        impl Position {
            pub fn deserialize(data: &[u8]) -> Option<Self> {
                <Self as BorshDeserialize>::try_from_slice(data).ok()
            }
        }
    }
}

pub mod accounts {
    pub mod pool {
        use super::super::{types::*, Pubkey};
        use borsh::BorshDeserialize;

        #[derive(BorshDeserialize, Clone, Debug, PartialEq, Eq)]
        pub struct Pool {
            pub pool_fees: PoolFeesStruct,
            pub token_a_mint: Pubkey,
            pub token_b_mint: Pubkey,
            pub token_a_vault: Pubkey,
            pub token_b_vault: Pubkey,
            pub whitelisted_vault: Pubkey,
            pub partner: Pubkey,
            pub liquidity: u128,
            pub padding: u128,
            pub protocol_a_fee: u64,
            pub protocol_b_fee: u64,
            pub partner_a_fee: u64,
            pub partner_b_fee: u64,
            pub sqrt_min_price: u128,
            pub sqrt_max_price: u128,
            pub sqrt_price: u128,
            pub activation_point: u64,
            pub activation_type: u8,
            pub pool_status: u8,
            pub token_a_flag: u8,
            pub token_b_flag: u8,
            pub collect_fee_mode: u8,
            pub pool_type: u8,
            pub padding_0: [u8; 2],
            pub fee_a_per_liquidity: [u8; 32],
            pub fee_b_per_liquidity: [u8; 32],
            pub permanent_lock_liquidity: u128,
            pub metrics: PoolMetrics,
            pub padding_1: [u64; 10],
            pub reward_infos: [RewardInfo; 2],
        }

        impl Pool {
            const DISCRIMINATOR: [u8; 8] = [0xf1, 0x9a, 0x6d, 0x04, 0x11, 0xb1, 0x6d, 0xbc];

            pub fn deserialize(data: &[u8]) -> Option<Self> {
                if data.len() < Self::DISCRIMINATOR.len() {
                    return None;
                }

                let (disc, rest) = data.split_at(Self::DISCRIMINATOR.len());
                if disc != Self::DISCRIMINATOR {
                    return None;
                }

                <Self as BorshDeserialize>::try_from_slice(rest).ok()
            }
        }
    }
}
