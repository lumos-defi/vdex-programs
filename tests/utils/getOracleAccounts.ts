import { PublicKey, AccountMeta } from '@solana/web3.js'
import { getProviderAndProgram } from './getProvider'
import { DexInfo } from './types'

export async function getOracleAccounts(dex: PublicKey): Promise<Array<AccountMeta>> {
  const { program } = getProviderAndProgram()
  const dexInfo = (await program.account.dex.fetch(dex)) as DexInfo

  const assetOracleAccounts = new Array<AccountMeta>()
  const marketOracleAccounts = new Array<AccountMeta>()

  dexInfo.assets
    .filter((a: { valid: boolean }) => a.valid)
    .forEach((a: { oracle: PublicKey }) => {
      assetOracleAccounts.push({
        isSigner: false,
        isWritable: false,
        pubkey: a.oracle,
      })
    })

  dexInfo.markets
    .filter((m: { valid: any }) => m.valid)
    .forEach((m: { oracle: any }) => {
      marketOracleAccounts.push({
        isSigner: false,
        isWritable: false,
        pubkey: m.oracle,
      })
    })

  return assetOracleAccounts.concat(marketOracleAccounts)
}
