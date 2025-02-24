use std::rc::Rc;
use anchor_client::{
    Program,
    solana_sdk::signature::Keypair,
};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use anchor_spl::token::TokenAccount;

#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone, Pod, Zeroable)]
pub struct PoolState {
    pub amm_config: Pubkey,
    pub pool_creator: Pubkey,
    pub token_0_vault: Pubkey,
    pub token_1_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub token_0_mint: Pubkey,
    pub token_1_mint: Pubkey,
    pub token_0_program: Pubkey,
    pub token_1_program: Pubkey,
    pub observation_key: Pubkey,
    pub auth_bump: u8,
    pub status: u8,
    pub lp_mint_decimals: u8,
    pub mint_0_decimals: u8,
    pub mint_1_decimals: u8,
    pub lp_supply: u64,
    pub protocol_fees_token_0: u64,
    pub protocol_fees_token_1: u64,
    pub fund_fees_token_0: u64,
    pub fund_fees_token_1: u64,
    pub open_time: u64,
    pub recent_epoch: u64,
    pub padding: [u64; 31],
}

impl anchor_lang::AccountDeserialize for PoolState {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        if buf.len() < 8 + std::mem::size_of::<PoolState>() {
            return Err(error!(ErrorCode::AccountDiscriminatorNotFound));
        }
        // Skip 8-byte discriminator
        *buf = &buf[8..];
        Ok(bytemuck::from_bytes::<PoolState>(buf).clone())
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        Self::try_deserialize(buf)
    }
}

impl PoolState {
    pub async fn get_vault_amounts(&self, spl_program: &Program<Rc<Keypair>>) -> std::result::Result<(u64, u64), anchor_client::ClientError> {
        let vault_0: TokenAccount = spl_program.account(self.token_0_vault).await?;
        let vault_1: TokenAccount = spl_program.account(self.token_1_vault).await?;
        Ok((vault_0.amount, vault_1.amount))
    }
}

pub async fn get_pool_state(
    raydium_program: &Program<Rc<Keypair>>,
    pool_address: Pubkey,
) -> std::result::Result<PoolState, anchor_client::ClientError> {
    raydium_program.account::<PoolState>(pool_address).await
} 