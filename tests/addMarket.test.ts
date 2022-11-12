import { Keypair } from '@solana/web3.js'
import { createAccountInstruction } from './utils/createAccountInstruction'
import { createDex } from './utils/createDex'
import { createMockOracle } from './utils/createMockOracle'
import { getProviderAndProgram } from './utils/getProvider'
import { BN } from '@project-serum/anchor'

describe('Test Create Market', () => {
  const { program } = getProviderAndProgram()

  const MARKET_SYMBOL = 'BTC/USDC'
  const DECIMALS = 8
  const MOCK_ORACLE_PRICE = 1
  const MOCK_ORACLE_PRICE_EXPO = 88
  const ORACLE_SOURCE = 0 // 0: mock,1: pyth
  const SIGNIFICANT_DECIMALS = 2 // 0.00
  const CHARGE_BORROW_FEE_INTERVAL = 3600
  const OPEN_FEE_RATE = 30 // 0.3% (30 / 10000)
  const CLOSE_FEE_RATE = 50 // 0.5%   (50 /  10000)
  const LIQUIDATE_FEE_RATE = 80 // 0.8%   (80 /  10000)

  const ASSET_INDEX = 0

  let dex: Keypair
  let orderBook: Keypair
  let orderPoolEntryPage: Keypair
  let authority: Keypair
  let mockOracle: Keypair

  beforeEach(async () => {
    orderBook = Keypair.generate()
    orderPoolEntryPage = Keypair.generate()
    authority = Keypair.generate()
    ;({ dex } = await createDex(authority))
    ;({ mockOracle } = await createMockOracle(authority, MOCK_ORACLE_PRICE, MOCK_ORACLE_PRICE_EXPO))
  })

  it('should add same market twice failed', async () => {
    await program.methods
      .addMarket(
        MARKET_SYMBOL,
        new BN(100),
        new BN(CHARGE_BORROW_FEE_INTERVAL),
        OPEN_FEE_RATE,
        CLOSE_FEE_RATE,
        LIQUIDATE_FEE_RATE,
        DECIMALS,
        ORACLE_SOURCE,
        ASSET_INDEX,
        SIGNIFICANT_DECIMALS
      )
      .accounts({
        dex: dex.publicKey,
        orderBook: orderBook.publicKey,
        orderPoolEntryPage: orderPoolEntryPage.publicKey,
        authority: authority.publicKey,
        oracle: mockOracle.publicKey,
      })
      .preInstructions([
        await createAccountInstruction(orderBook, 128 * 1024),
        await createAccountInstruction(orderPoolEntryPage, 128 * 1024),
      ])
      .signers([authority, orderBook, orderPoolEntryPage])
      .rpc()

    expect(async () => {
      await program.methods
        .addMarket(
          MARKET_SYMBOL,
          new BN(100),
          new BN(CHARGE_BORROW_FEE_INTERVAL),
          OPEN_FEE_RATE,
          CLOSE_FEE_RATE,
          LIQUIDATE_FEE_RATE,
          DECIMALS,
          ORACLE_SOURCE,
          ASSET_INDEX,
          SIGNIFICANT_DECIMALS
        )
        .accounts({
          dex: dex.publicKey,
          orderBook: orderBook.publicKey,
          orderPoolEntryPage: orderPoolEntryPage.publicKey,
          authority: authority.publicKey,
          oracle: mockOracle.publicKey,
        })
        .signers([authority])
        .rpc()
    }).rejects.toThrow('Duplicate market name')
  })

  it('should add market successfully', async () => {
    await program.methods
      .addMarket(
        MARKET_SYMBOL,
        new BN(100),
        new BN(CHARGE_BORROW_FEE_INTERVAL),
        OPEN_FEE_RATE,
        CLOSE_FEE_RATE,
        LIQUIDATE_FEE_RATE,
        DECIMALS,
        ORACLE_SOURCE,
        ASSET_INDEX,
        SIGNIFICANT_DECIMALS
      )
      .accounts({
        dex: dex.publicKey,
        orderBook: orderBook.publicKey,
        orderPoolEntryPage: orderPoolEntryPage.publicKey,
        authority: authority.publicKey,
        oracle: mockOracle.publicKey,
      })
      .preInstructions([
        await createAccountInstruction(orderBook, 128 * 1024),
        await createAccountInstruction(orderPoolEntryPage, 128 * 1024),
      ])
      .signers([authority, orderBook, orderPoolEntryPage])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo).toMatchObject({
      magic: expect.toBNEqual(0x6666),
      marketsNumber: 1,
    })

    const marketInfo = dexInfo.markets[0]
    expect(marketInfo).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC/USDC\0\0\0\0\0\0\0\0'),
      decimals: DECIMALS,
      orderBook: orderBook.publicKey,
      orderPoolEntryPage: orderPoolEntryPage.publicKey,
      oracle: mockOracle.publicKey,
      minimumOpenAmount: expect.toBNEqual(100),
      chargeBorrowFeeInterval: expect.toBNEqual(CHARGE_BORROW_FEE_INTERVAL),
      openFeeRate: OPEN_FEE_RATE,
      closeFeeRate: CLOSE_FEE_RATE,
      liquidateFeeRate: LIQUIDATE_FEE_RATE,
      significantDecimals: SIGNIFICANT_DECIMALS,
      orderPoolRemainingPagesNumber: 0,
    })
  })
})
