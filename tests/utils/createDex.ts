import { TokenInstructions } from '@project-serum/serum'
import { Keypair } from '@solana/web3.js'
import { createAccountInstruction } from './createAccountInstruction'
import { createMint } from './createMint'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createDex(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()
  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const matchQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()
  const rewardMint = TokenInstructions.WRAPPED_SOL_MINT
  const VLP_DECIMALS = 6
  const USDC_MINT_DECIMALS = 6

  await airdrop(provider, authority.publicKey, 10000000000)
  const usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)

  await program.methods
    .initDex(VLP_DECIMALS)
    .accounts({
      dex: dex.publicKey,
      usdcMint,
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
      rewardMint,
    })
    .preInstructions([
      await program.account.dex.createInstruction(dex),
      await createAccountInstruction(eventQueue, 128 * 1024),
      await createAccountInstruction(matchQueue, 128 * 1024),
      await createAccountInstruction(userListEntryPage, 128 * 1024),
    ])
    .signers([authority, dex, eventQueue, matchQueue, userListEntryPage])
    .rpc()

  return { dex }
}
