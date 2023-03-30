import { Keypair } from '@solana/web3.js'
import { getProviderAndProgram } from './utils/getProvider'
import { createDexFull } from './utils/createDexFull'
import { BN } from '@project-serum/anchor'

describe('Update Price', () => {
  const { program } = getProviderAndProgram()
  let dex: Keypair
  let priceFeed: Keypair
  let authority: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    priceFeed = Keypair.generate()
    ;({ dex, priceFeed } = await createDexFull(authority))
  })

  it('should update price account successfully', async () => {
    for (let i = 0; i <= 5; i++) {
      const prices = [new BN((i + 1) * 10_000_000_000), new BN((i + 1) * 100_000_000), ...new Array(14).fill(new BN(0))]
      await program.methods
        .updatePrice(prices)
        .accounts({
          dex: dex.publicKey,
          priceFeed: priceFeed.publicKey,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc()

      await new Promise((resolve) => setTimeout(resolve, 1000))
    }

    const priceFeedInfo = await program.account.priceFeed.fetch(priceFeed.publicKey)

    console.log(priceFeedInfo)
    console.log(0, priceFeedInfo.prices[0])
    console.log(1, priceFeedInfo.prices[1])
    console.log(2, priceFeedInfo.prices[2])

    expect(priceFeedInfo.prices[0]).toMatchObject({
      assetPrices: expect.arrayContaining([
        expect.objectContaining({
          price: expect.toBNEqual(60_000_000_000),
        }),
        expect.objectContaining({
          price: expect.toBNEqual(50_000_000_000),
        }),
        expect.objectContaining({
          price: expect.toBNEqual(40_000_000_000),
        }),
        expect.objectContaining({
          price: expect.toBNEqual(30_000_000_000),
        }),
        expect.objectContaining({
          price: expect.toBNEqual(20_000_000_000),
        }),
      ]),
    })
  })
})
