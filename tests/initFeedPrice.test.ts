import { Keypair } from '@solana/web3.js'
import { createDex } from './utils/createDex'
import { getProviderAndProgram } from './utils/getProvider'

describe('Init Feed Price', () => {
  const { program } = getProviderAndProgram()
  let dex: Keypair
  let feedPrice: Keypair
  let authority: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    feedPrice = Keypair.generate()
    ;({ dex } = await createDex(authority))
  })

  it('should init feed price account successfully', async () => {
    await program.methods
      .initFeedPrice()
      .accounts({
        dex: dex.publicKey,
        feedPrice: feedPrice.publicKey,
        authority: authority.publicKey,
      })
      .preInstructions([await program.account.feedPrice.createInstruction(feedPrice)])
      .signers([authority, feedPrice])
      .rpc()

    const feedPriceInfo = await program.account.feedPrice.fetch(feedPrice.publicKey)

    expect(feedPriceInfo).toMatchObject({
      magic: expect.toBNEqual(0x666a),
      authority: authority.publicKey,
      prices: expect.arrayContaining([
        expect.objectContaining({
          assetPrice: expect.toBNEqual(0),
          updateTime: expect.toBNEqual(0),
          assetIndex: 0,
          valid: false,
        }),
      ]),
    })
  })
})
