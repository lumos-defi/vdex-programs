import { TokenInstructions } from '@project-serum/serum'
import { TOKEN_PROGRAM_ID } from '@project-serum/serum/lib/token-instructions'
import * as anchor from '@project-serum/anchor'
import { getProviderAndProgram } from './getProvider'

export async function createTokenAccount(mint, owner) {
  const { provider } = getProviderAndProgram()
  const vault = anchor.web3.Keypair.generate()
  const tx = new anchor.web3.Transaction()
  tx.add(...(await createTokenAccountInstruction(provider, vault.publicKey, mint, owner, undefined)))
  await provider.sendAndConfirm(tx, [vault])
  return vault.publicKey
}

async function createTokenAccountInstruction(provider, newAccountPubkey, mint, owner, lamports) {
  if (lamports === undefined) {
    lamports = await provider.connection.getMinimumBalanceForRentExemption(165)
  }
  return [
    anchor.web3.SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey,
      space: 165,
      lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    TokenInstructions.initializeAccount({
      account: newAccountPubkey,
      mint,
      owner,
    }),
  ]
}
