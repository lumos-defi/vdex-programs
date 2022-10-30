use anchor_lang::prelude::*;
use errors::*;

pub mod errors;

declare_id!("2aJZ6AufDU5NRzXLg5Ww4S4Nf2tx7xZDQD6he2gjsKyq");

#[program]
pub mod dex_program {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> DexResult {
        Ok(())
    }

    pub fn add_asset(_ctx: Context<AddAsset>) -> DexResult {
        Ok(())
    }

    pub fn add_market(_ctx: Context<AddMarket>) -> DexResult {
        Ok(())
    }

    pub fn add_liquidity(_ctx: Context<AddLiquidity>) -> DexResult {
        Ok(())
    }

    pub fn remove_liquidity(_ctx: Context<RemoveLiquidity>) -> DexResult {
        Ok(())
    }

    pub fn swap(_ctx: Context<Swap>) -> DexResult {
        Ok(())
    }

    pub fn open_position(_ctx: Context<OpenPosition>) -> DexResult {
        Ok(())
    }

    pub fn close_position(_ctx: Context<ClosePosition>) -> DexResult {
        Ok(())
    }

    pub fn close_all_positions(_ctx: Context<CloseAllPositions>) -> DexResult {
        Ok(())
    }

    pub fn new_limit_order(_ctx: Context<NewLimitOrder>) -> DexResult {
        Ok(())
    }

    pub fn cancel_limit_order(_ctx: Context<CancelLimitOrder>) -> DexResult {
        Ok(())
    }

    pub fn cancel_all_limit_orders(_ctx: Context<CancelAllLimitOrders>) -> DexResult {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddAsset<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddMarket<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseAllPositions<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct NewLimitOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelLimitOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelAllLimitOrders<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
}
