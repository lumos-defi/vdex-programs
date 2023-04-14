import * as anchor from '@project-serum/anchor'
import { TransactionInstruction, Signer } from '@solana/web3.js'
import { getProviderAndProgram } from './getProvider'

export async function createAccountInstruction(signer: Signer, size: number): Promise<TransactionInstruction> {
  const { provider, program } = getProviderAndProgram()

  return anchor.web3.SystemProgram.createAccount({
    fromPubkey: provider.wallet.publicKey,
    newAccountPubkey: signer.publicKey,
    space: size,
    lamports: await provider.connection.getMinimumBalanceForRentExemption(size),
    programId: program.programId,
  })
}

export async function createAccount(signer: Signer, size: number) {
  const { provider } = getProviderAndProgram()

  const instruction = await createAccountInstruction(signer, size)

  const tx = new anchor.web3.Transaction()
  tx.add(instruction)

  await provider.sendAndConfirm(tx, [signer])
}
