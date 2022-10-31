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
