import { PublicKey, Keypair, SystemProgram } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createUserState(user: Keypair, dex: Keypair) {
  const { provider, program } = getProviderAndProgram()
  const orderSlotCount = 32
  const positionSlotCount = 32

  await airdrop(provider, user.publicKey, 10000000000)

  const [userState] = await PublicKey.findProgramAddress(
    [dex.publicKey.toBuffer(), user.publicKey.toBuffer()],
    program.programId
  )

  await program.methods
    .createUserState(orderSlotCount, positionSlotCount)
    .accounts({
      userState: userState,
      dex: dex.publicKey,
      authority: user.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([user])
    .rpc()

  return userState
}