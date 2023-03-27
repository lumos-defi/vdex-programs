import { PublicKey, Keypair, SystemProgram } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
describe('Test Create User State', () => {
  const { program, provider } = getProviderAndProgram()

  let dex: Keypair, alice: Keypair, userStatePublicKey: PublicKey

  describe('Create User State', () => {
    beforeEach(async () => {
      dex = Keypair.generate()
      alice = Keypair.generate()

      await airdrop(provider, alice.publicKey, 10000000000)

      //gen user order state with seeds
      ;[userStatePublicKey] = PublicKey.findProgramAddressSync(
        [dex.publicKey.toBuffer(), alice.publicKey.toBuffer()],
        program.programId
      )
    })

    it('should create same publickey with same buffer successfully', async () => {
      const [publicKey1, nonce1] = PublicKey.findProgramAddressSync(
        [dex.publicKey.toBuffer(), alice.publicKey.toBuffer()],
        program.programId
      )

      const [publicKey2, nonce2] = PublicKey.findProgramAddressSync(
        [dex.publicKey.toBuffer(), alice.publicKey.toBuffer()],
        program.programId
      )

      expect(publicKey1.toString()).toBe(publicKey2.toString())
      expect(nonce1).toEqual(nonce2)
    })

    it('should create user state account successfully', async () => {
      const orderSlotCount = 32
      const positionSlotCount = 32
      const assetSlotCount = 8
      await program.methods
        .createUserState(orderSlotCount, positionSlotCount, assetSlotCount)
        .accounts({
          userState: userStatePublicKey,
          dex: dex.publicKey,
          authority: alice.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([alice])
        .rpc()

      const userState = await provider.connection.getAccountInfo(userStatePublicKey)

      console.log('userState', userState)
    })
  })
})
