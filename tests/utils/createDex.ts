import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccountInstruction } from './createAccountInstruction'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createDex(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()
  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const matchQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()
  const VLP_DECIMALS = 6
  let vlpMint: PublicKey
  let vlpMintAuthority: PublicKey
  let vlpMintNonce: number

  await airdrop(provider, authority.publicKey, 10000000000)

  //gen vlp mint with seeds
  // eslint-disable-next-line prefer-const
  ;[vlpMint] = await PublicKey.findProgramAddress([dex.publicKey.toBuffer(), Buffer.from('vlp')], program.programId)
  // eslint-disable-next-line prefer-const
  ;[vlpMintAuthority, vlpMintNonce] = await PublicKey.findProgramAddress(
    [dex.publicKey.toBuffer(), vlpMint.toBuffer()],
    program.programId
  )

  await program.methods
    .initDex(VLP_DECIMALS, vlpMintNonce)
    .accounts({
      dex: dex.publicKey,
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
      vlpMint: vlpMint,
      vlpMintAuthority: vlpMintAuthority,
    })
    .preInstructions([
      await program.account.dex.createInstruction(dex),
      await createAccountInstruction(eventQueue, 128 * 1024),
      await createAccountInstruction(matchQueue, 128 * 1024),
      await createAccountInstruction(userListEntryPage, 128 * 1024),
    ])
    .signers([authority, dex, eventQueue, matchQueue, userListEntryPage])
    .rpc()

  return { dex, vlpMint, vlpMintAuthority }
}
