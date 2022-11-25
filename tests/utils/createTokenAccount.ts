import { TokenInstructions } from '@project-serum/serum'
import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import * as anchor from '@project-serum/anchor'
import { getProviderAndProgram } from './getProvider'
import { PublicKey, Keypair } from '@solana/web3.js'

export async function createTokenAccount(mint, owner) {
  const { provider } = getProviderAndProgram()
  const vault = anchor.web3.Keypair.generate()
  const tx = new anchor.web3.Transaction()
  tx.add(
    ...(await createTokenAccountInstruction(
      provider,
      provider.wallet.publicKey,
      vault.publicKey,
      mint,
      owner,
      undefined
    ))
  )
  await provider.sendAndConfirm(tx, [vault])
  return vault.publicKey
}

export async function createWSOLAccount(owner: Keypair, lamports: number): Promise<[PublicKey, number]> {
  const { provider } = getProviderAndProgram()
  const vault = anchor.web3.Keypair.generate()
  const tx = new anchor.web3.Transaction()

  const accountLamports = await provider.connection.getMinimumBalanceForRentExemption(165)
  tx.add(
    ...(await createTokenAccountInstruction(
      provider,
      owner.publicKey,
      vault.publicKey,
      TokenInstructions.WRAPPED_SOL_MINT,
      owner.publicKey,
      lamports + accountLamports
    ))
  )
  await provider.sendAndConfirm(tx, [owner, vault])
  return [vault.publicKey, accountLamports]
}

async function createTokenAccountInstruction(provider, fromPubkey, newAccountPubkey, mint, owner, lamports) {
  if (lamports === undefined) {
    lamports = await provider.connection.getMinimumBalanceForRentExemption(165)
  }
  return [
    anchor.web3.SystemProgram.createAccount({
      fromPubkey,
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
