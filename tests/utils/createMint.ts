import * as anchor from '@project-serum/anchor'
import { TokenInstructions } from '@project-serum/serum'
import { getProviderAndProgram } from './getProvider'

const TOKEN_PROGRAM_ID = new anchor.web3.PublicKey(TokenInstructions.TOKEN_PROGRAM_ID.toString())
async function createMintInstructions(provider, authority, mint, decimals) {
  const instructions = [
    anchor.web3.SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: mint,
      space: 82,
      lamports: await provider.connection.getMinimumBalanceForRentExemption(82),
      programId: TOKEN_PROGRAM_ID,
    }),
    TokenInstructions.initializeMint({
      mint,
      decimals: decimals,
      mintAuthority: authority,
    }),
  ]
  return instructions
}

export async function createMint(authority, decimals) {
  const { provider } = getProviderAndProgram()
  if (authority === undefined) {
    authority = provider.wallet.publicKey
  }
  const mint = anchor.web3.Keypair.generate()
  const instructions = await createMintInstructions(provider, authority, mint.publicKey, decimals)

  const tx = new anchor.web3.Transaction()
  tx.add(...instructions)

  await provider.sendAndConfirm(tx, [mint])

  return mint.publicKey
}
