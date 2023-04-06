import { Keypair } from '@solana/web3.js'
import { createDex } from './utils/createDex'
import { getProviderAndProgram } from './utils/getProvider'

describe('Init Feed Price', () => {
  const { program } = getProviderAndProgram()
  let dex: Keypair
  let priceFeed: Keypair
  let authority: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    priceFeed = Keypair.generate()
    ;({ dex } = await createDex(authority))
  })

  it('should init price feed account successfully', async () => {
    await program.methods
      .initPriceFeed()
      .accounts({
        dex: dex.publicKey,
        priceFeed: priceFeed.publicKey,
        authority: authority.publicKey,
      })
      .preInstructions([await program.account.priceFeed.createInstruction(priceFeed)])
      .signers([authority, priceFeed])
      .rpc()

    const priceFeedInfo = await program.account.priceFeed.fetch(priceFeed.publicKey)

    console.log(priceFeedInfo, priceFeedInfo.prices[0], priceFeedInfo.prices[0].assetPrices[0])
    expect(priceFeedInfo).toMatchObject({
      magic: expect.toBNEqual(0x666a),
      authority: authority.publicKey,
      prices: expect.arrayContaining([
        expect.objectContaining({
          assetPrices: expect.arrayContaining([
            expect.objectContaining({
              price: expect.toBNEqual(0),
              updateTime: expect.toBNEqual(0),
            }),
          ]),
          cursor: 0,
        }),
      ]),
      lastUpdateTime: expect.toBNEqual(0),
    })
  })
})
