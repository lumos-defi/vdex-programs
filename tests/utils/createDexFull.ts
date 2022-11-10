import { BN } from '@project-serum/anchor'
import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccountInstruction } from './createAccountInstruction'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { createMockOracle } from './createMockOracle'
import { createTokenAccount } from './createTokenAccount'
import { airdrop, getProviderAndProgram } from './getProvider'
import { createMint } from './createMint'

export async function createDexFull(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()

  //asset
  const ASSET_SYMBOL = 'BTC'
  const ASSET_MINT_DECIMAL = 9
  const BORROWED_FEE_RATE = 100 // 1-10000, the percentage will be XXXX_RATE / 10000
  const ADD_LIQUIDITY_FEE_RATE = 100
  const REMOVE_LIQUIDITY_FEE_RATE = 100
  const TARGET_WEIGHT = 100 //1-1000, the percentage will be weight / 1000

  //oracle
  const MOCK_ORACLE_PRICE = 20_000_000_000 //$20000
  const MOCK_ORACLE_PRICE_EXPO = 6
  const ORACLE_SOURCE = 0 // 0:mock,1:pyth

  //market
  const MARKET_SYMBOL = 'BTC/USDC'
  const DECIMALS = 9
  const SIGNIFICANT_DECIMALS = 2 // 0.00
  const CHARGE_BORROW_FEE_INTERVAL = 3600
  const OPEN_FEE_RATE = 30 // 0.3% (30 / 10000)
  const CLOSE_FEE_RATE = 50 // 0.5%   (50 /  10000)
  const BORROW_FEE_RATE = 10 // 0.1%   (10 /  10000)
  const ASSET_INDEX = 0
  const USDC_MINT_DECIMALS = 6

  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const matchQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()
  const VLP_DECIMALS = 6

  const longOrderBook = Keypair.generate()
  const shortOrderBook = Keypair.generate()
  const orderPoolEntryPage = Keypair.generate()

  await airdrop(provider, authority.publicKey, 10000000000)
  const usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)
  //gen vlp mint with seeds
  const [vlpMint] = await PublicKey.findProgramAddress(
    [dex.publicKey.toBuffer(), Buffer.from('vlp')],
    program.programId
  )

  const [vlpMintAuthority, vlpMintNonce] = await PublicKey.findProgramAddress(
    [dex.publicKey.toBuffer(), vlpMint.toBuffer()],
    program.programId
  )

  //init dex
  await program.methods
    .initDex(VLP_DECIMALS, vlpMintNonce)
    .accounts({
      dex: dex.publicKey,
      usdcMint,
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

  //add asset
  const { mockOracle } = await createMockOracle(authority, MOCK_ORACLE_PRICE, MOCK_ORACLE_PRICE_EXPO)

  //mint
  const assetMint = await Token.createMint(
    provider.connection,
    authority,
    authority.publicKey,
    null,
    ASSET_MINT_DECIMAL,
    TOKEN_PROGRAM_ID
  )

  //pda
  const [programSigner, nonce] = await PublicKey.findProgramAddress(
    [assetMint.publicKey.toBuffer(), dex.publicKey.toBuffer()],
    program.programId
  )

  //vault
  const assetVault = await createTokenAccount(assetMint.publicKey, programSigner)

  await program.methods
    .addAsset(
      ASSET_SYMBOL,
      ASSET_MINT_DECIMAL,
      nonce,
      ORACLE_SOURCE,
      BORROWED_FEE_RATE,
      ADD_LIQUIDITY_FEE_RATE,
      REMOVE_LIQUIDITY_FEE_RATE,
      TARGET_WEIGHT
    )
    .accounts({
      dex: dex.publicKey,
      mint: assetMint.publicKey,
      oracle: mockOracle.publicKey,
      vault: assetVault,
      programSigner: programSigner,
      authority: authority.publicKey,
    })
    .signers([authority])
    .rpc()

  //add market
  await program.methods
    .addMarket(
      MARKET_SYMBOL,
      new BN(100),
      new BN(CHARGE_BORROW_FEE_INTERVAL),
      OPEN_FEE_RATE,
      CLOSE_FEE_RATE,
      BORROW_FEE_RATE,
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

  return {
    dex,
    assetMint,
    assetVault,
    programSigner,
    nonce,
    authority,
    eventQueue,
    userListEntryPage,
    longOrderBook,
    shortOrderBook,
    orderPoolEntryPage,
    vlpMint,
    vlpMintAuthority,
    MOCK_ORACLE_PRICE,
    ADD_LIQUIDITY_FEE_RATE,
  }
}
