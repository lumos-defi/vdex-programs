import { PublicKey } from '@solana/web3.js'

export type DexInfo = {
  magic: number
  assets: Array<AssetInfo>
  markets: Array<MarketInfo>
}

export type AssetInfo = {
  valid: boolean
  symbol: string
  decimals: number
  oracleSource: OracleSource
  oracle: PublicKey
  mint: PublicKey
  vault: PublicKey
  programSigner: PublicKey
}

export type MarketInfo = {
  valid: boolean
  baseDecimals: number
  oracleSource: number
  oracle: PublicKey
}

export enum OracleSource {
  MOCK = 0,
  PYTH = 1,
  STABLECOIN = 2,
}
