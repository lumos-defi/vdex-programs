import { PublicKey, Keypair, SystemProgram } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createUserState(user: Keypair, dex: Keypair) {
  const { provider, program } = getProviderAndProgram()
  const orderSlotCount = 16
  const positionSlotCount = 16
  const assetSlotCount = 10

  await airdrop(provider, user.publicKey, 10000000000)

  const [userState] = PublicKey.findProgramAddressSync(
    [dex.publicKey.toBuffer(), user.publicKey.toBuffer()],
    program.programId
  )

  await program.methods
    .createUserState(orderSlotCount, positionSlotCount, assetSlotCount)
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
