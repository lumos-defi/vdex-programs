import { Keypair } from '@solana/web3.js'
import { createAccountInstruction } from './utils/createAccountInstruction'
import { createDex } from './utils/createDex'
import { createMockOracle } from './utils/createMockOracle'
import { getProviderAndProgram } from './utils/getProvider'

describe('Test Create Market', () => {
  const { program } = getProviderAndProgram()

  const MARKET_SYMBOL = 'BTC/USDC'
  const DECIMALS = 8
  const MOCK_ORACLE_PRICE = 1
  const MOCK_ORACLE_PRICE_EXPO = 88
  const ORACLE_SOURCE = 0 // 0: mock,1: pyth
  const SIGNIFICANT_DECIMALS = 2 // 0.00
  const OPEN_FEE_RATE = 30 // 0.03% (30 / 100000)
  const CLOSE_FEE_RATE = 50 // 0.05%   (50 /  100000)
  const ASSET_INDEX = 0

  let dex: Keypair
  let longOrderBook: Keypair
  let shortOrderBook: Keypair
  let orderPoolEntryPage: Keypair
  let authority: Keypair
  let mockOracle: Keypair

  beforeEach(async () => {
    longOrderBook = Keypair.generate()
    shortOrderBook = Keypair.generate()
    orderPoolEntryPage = Keypair.generate()
    authority = Keypair.generate()
    ;({ dex } = await createDex(authority))
    ;({ mockOracle } = await createMockOracle(authority, MOCK_ORACLE_PRICE, MOCK_ORACLE_PRICE_EXPO))
  })

  it('should add same market twice failed', async () => {
    await program.methods
      .addMarket(
        MARKET_SYMBOL,
        OPEN_FEE_RATE,
        CLOSE_FEE_RATE,
        DECIMALS,
        ORACLE_SOURCE,
        ASSET_INDEX,
        SIGNIFICANT_DECIMALS
      )
      .accounts({
        dex: dex.publicKey,
        longOrderBook: longOrderBook.publicKey,
        shortOrderBook: shortOrderBook.publicKey,
        orderPoolEntryPage: orderPoolEntryPage.publicKey,
        authority: authority.publicKey,
        oracle: mockOracle.publicKey,
      })
      .preInstructions([
        await createAccountInstruction(longOrderBook, 128 * 1024),
        await createAccountInstruction(shortOrderBook, 128 * 1024),
        await createAccountInstruction(orderPoolEntryPage, 128 * 1024),
      ])
      .signers([authority, longOrderBook, shortOrderBook, orderPoolEntryPage])
      .rpc()

    expect(async () => {
      await program.methods
        .addMarket(
          MARKET_SYMBOL,
          OPEN_FEE_RATE,
          CLOSE_FEE_RATE,
          DECIMALS,
          ORACLE_SOURCE,
          ASSET_INDEX,
          SIGNIFICANT_DECIMALS
        )
        .accounts({
          dex: dex.publicKey,
          longOrderBook: longOrderBook.publicKey,
          shortOrderBook: shortOrderBook.publicKey,
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
        OPEN_FEE_RATE,
        CLOSE_FEE_RATE,
        DECIMALS,
        ORACLE_SOURCE,
        ASSET_INDEX,
        SIGNIFICANT_DECIMALS
      )
      .accounts({
        dex: dex.publicKey,
        longOrderBook: longOrderBook.publicKey,
        shortOrderBook: shortOrderBook.publicKey,
        orderPoolEntryPage: orderPoolEntryPage.publicKey,
        authority: authority.publicKey,
        oracle: mockOracle.publicKey,
      })
      .preInstructions([
        await createAccountInstruction(longOrderBook, 128 * 1024),
        await createAccountInstruction(shortOrderBook, 128 * 1024),
        await createAccountInstruction(orderPoolEntryPage, 128 * 1024),
      ])
      .signers([authority, longOrderBook, shortOrderBook, orderPoolEntryPage])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo).toMatchObject({
      magic: expect.toBNEqual(0x6666),
      marketsNumber: 1,
    })

    expect(dexInfo.markets[0]).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC/USDC\0\0\0\0\0\0\0\0'),
      decimals: DECIMALS,
      longOrderBook: longOrderBook.publicKey,
      shortOrderBook: shortOrderBook.publicKey,
      orderPoolEntryPage: orderPoolEntryPage.publicKey,
      oracle: mockOracle.publicKey,
      openFeeRate: OPEN_FEE_RATE,
      closeFeeRate: CLOSE_FEE_RATE,
      significantDecimals: SIGNIFICANT_DECIMALS,
    })
  })
})
