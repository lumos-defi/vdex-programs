use std::convert::TryFrom;

use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

use crate::{
    dex::{state::*, OracleSource},
    errors::{DexError, DexResult},
};

#[derive(Accounts)]
pub struct AddAsset<'info> {
    #[account(
        mut,
        has_one = authority,
    )]
    pub dex: AccountLoader<'info, Dex>,

    /// CHECK: Mint asset to trade
    mint: AccountInfo<'info>,

    /// CHECK
    pub oracle: UncheckedAccount<'info>,

    /// CHECK: Vault for locking asset
    #[account(constraint = vault.mint == *mint.key  @DexError::InvalidMint)]
    vault: Box<Account<'info, TokenAccount>>,

    /// CHECK: PDA to access program owned vault
    pub program_signer: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

pub fn handler(
    ctx: Context<AddAsset>,
    symbol: String,
    decimals: u8,
    nonce: u8,
    oracle_source: u8,
    borrow_fee_rate: u16,
    add_liquidity_fee_rate: u16,
    remove_liquidity_fee_rate: u16,
    swap_fee_rate: u16,
    target_weight: u16,
) -> DexResult {
    OracleSource::try_from(oracle_source).map_err(|_| DexError::InvalidOracleSource)?;

    let (program_signer, program_signer_nonce) = Pubkey::find_program_address(
        &[
            &ctx.accounts.mint.key.to_bytes(),
            &ctx.accounts.dex.to_account_info().key.to_bytes(),
        ],
        ctx.program_id,
    );

    require_eq!(nonce, program_signer_nonce, DexError::InvalidProgramSigner);
    require_eq!(
        ctx.accounts.vault.owner,
        program_signer,
        DexError::InvalidProgramSigner
    );

    let dex = &mut ctx.accounts.dex.load_mut()?;

    let mut asset_symbol: [u8; 16] = Default::default();
    let given_name = symbol.as_bytes();
    let assets = &dex.assets;

    asset_symbol[..given_name.len()].copy_from_slice(given_name);
    if assets.iter().any(|asset| {
        asset.symbol == asset_symbol
            || asset.mint == ctx.accounts.mint.key()
            || asset.vault == ctx.accounts.vault.key()
    }) {
        return Err(error!(DexError::DuplicateAsset));
    }

    let asset_index = dex.assets_number as usize;

    require_neq!(
        asset_index,
        dex.assets.len(),
        DexError::InsufficientAssetIndex
    );

    let asset = AssetInfo {
        symbol: asset_symbol,
        mint: ctx.accounts.mint.key(),
        oracle: ctx.accounts.oracle.key(),
        vault: ctx.accounts.vault.to_account_info().key(),
        program_signer: ctx.accounts.program_signer.key(),
        liquidity_amount: 0,
        collateral_amount: 0,
        borrowed_amount: 0,
        fee_amount: 0,
        add_liquidity_fee: 0,
        remove_liquidity_fee: 0,
        swap_fee_rate,
        borrow_fee_rate,
        add_liquidity_fee_rate,
        remove_liquidity_fee_rate,
        target_weight,
        valid: true,
        decimals,
        nonce,
        oracle_source,
        padding: [0; 250],
    };

    dex.assets[asset_index] = asset;
    dex.assets_number += 1;

    if dex.usdc_mint == ctx.accounts.mint.key() {
        dex.usdc_asset_index = asset_index as u8;
    }

    Ok(())
}
