#![allow(dead_code)]

#[derive(Clone, Copy, Debug)]
pub enum DexMarket {
    BTC = 0,
    ETH = 1,
    SOL = 2,
}

pub enum DexAsset {
    USDC = 0,
    BTC = 1,
    ETH = 2,
    SOL = 3,
}
