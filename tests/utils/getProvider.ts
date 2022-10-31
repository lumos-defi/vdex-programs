import { Program } from '@project-serum/anchor'
import * as anchor from '@project-serum/anchor'
import { DexProgram } from '../../target/types/dex_program'

export function getProviderAndProgram() {
  const provider = anchor.AnchorProvider.local()
  anchor.setProvider(anchor.AnchorProvider.env())
  const program = anchor.workspace.DexProgram as Program<DexProgram>

  return { provider, program }
}

export async function airdrop(provider: anchor.AnchorProvider, receipt: anchor.web3.PublicKey, lamports: number) {
  // Airdropping tokens to a receipt.
  await provider.connection.confirmTransaction(await provider.connection.requestAirdrop(receipt, lamports), 'confirmed')
}

export const getBalance = async (provider: anchor.AnchorProvider, receipt: anchor.web3.PublicKey) => {
  const lamports = await provider.connection.getBalance(receipt)
  console.log(
    'Account',
    receipt.toBase58(),
    'containing',
    lamports / anchor.web3.LAMPORTS_PER_SOL,
    'SOL to pay for fees'
  )
  return lamports
}
