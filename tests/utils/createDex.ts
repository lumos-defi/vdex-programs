import { Keypair } from '@solana/web3.js'
import { createAccountInstruction } from './createAccountInstruction'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createDex(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()
  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()

  await airdrop(provider, authority.publicKey, 10000000000)

  await program.methods
    .initDex()
    .accounts({
      dex: dex.publicKey,
      eventQueue: eventQueue.publicKey,
      authority: authority.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
    })
    .preInstructions([
      await program.account.dex.createInstruction(dex),
      await createAccountInstruction(eventQueue, 128 * 1024),
      await createAccountInstruction(userListEntryPage, 128 * 1024),
    ])
    .signers([authority, dex, eventQueue, userListEntryPage])
    .rpc()

  return { dex }
}
