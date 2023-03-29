import { BN } from '@project-serum/anchor'
import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccountInstruction } from './createAccountInstruction'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { createMockOracle } from './createMockOracle'
import { createTokenAccount } from './createTokenAccount'
import { airdrop, getProviderAndProgram } from './getProvider'
import { createMint } from './createMint'
import { TokenInstructions } from '@project-serum/serum'

export async function createDexFull(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()

  //BTC asset
  const BTC_SYMBOL = 'BTC'
  const BTC_MINT_DECIMAL = 9
  const BORROW_FEE_RATE = 100 // 1-10000, the percentage will be XXXX_RATE / 10000
  const ADD_LIQUIDITY_FEE_RATE = 100
  const REMOVE_LIQUIDITY_FEE_RATE = 100
  const SWAP_FEE_RATE = 100
  const TARGET_WEIGHT = 100 //1-1000, the percentage will be weight / 1000
  //BTC oracle
  const BTC_ORACLE_PRICE = 20_000_000_000 //$20000
  const BTC_ORACLE_PRICE_EXPO = 6
  const BTC_ORACLE_SOURCE = 0 // 0:mock,1:pyth

  //SOL asset
  const SOL_SYMBOL = 'SOL'
  const SOL_MINT_DECIMAL = 9
  //SOL oracle
  const SOL_ORACLE_PRICE = 20_000_000 //$15
  const SOL_ORACLE_PRICE_EXPO = 6
  const SOL_ORACLE_SOURCE = 0 // 0:mock,1:pyth

  //market
  const MARKET_SYMBOL = 'BTC/USDC'
  const DECIMALS = 9
  const SIGNIFICANT_DECIMALS = 2 // 0.00
  const CHARGE_BORROW_FEE_INTERVAL = 3600
  const OPEN_FEE_RATE = 30 // 0.3% (30 / 10000)
  const CLOSE_FEE_RATE = 50 // 0.5%   (50 /  10000)
  const LIQUIDATE_FEE_RATE = 80
  const ASSET_INDEX = 0
  const USDC_MINT_DECIMALS = 6
  const MINIMUM_COLLATERAL = 100
  const MAX_LEVERAGE = 30_000 //10 (30_000 / 1_000)

  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const matchQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()
  const diOption = Keypair.generate()
  const VLP_DECIMALS = 6

  const orderBook = Keypair.generate()
  const orderPoolEntryPage = Keypair.generate()
  const priceFeed = Keypair.generate()

  await airdrop(provider, authority.publicKey, 10000000000)
  const usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)

  //init dex
  await program.methods
    .initDex(VLP_DECIMALS, 30)
    .accounts({
      dex: dex.publicKey,
      usdcMint,
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
      rewardMint: TokenInstructions.WRAPPED_SOL_MINT,
      diOption: diOption.publicKey,
    })
    .preInstructions([
      await program.account.dex.createInstruction(dex),
      await createAccountInstruction(eventQueue, 128 * 1024),
      await createAccountInstruction(matchQueue, 128 * 1024),
      await createAccountInstruction(userListEntryPage, 128 * 1024),
      await createAccountInstruction(diOption, 128 * 1024),
    ])
    .signers([authority, dex, eventQueue, matchQueue, userListEntryPage, diOption])
    .rpc()

  //Add BTC asset
  const btcOracle = await createMockOracle(authority, BTC_ORACLE_PRICE, BTC_ORACLE_PRICE_EXPO)

  //mint
  const assetMint = await Token.createMint(
    provider.connection,
    authority,
    authority.publicKey,
    null,
    BTC_MINT_DECIMAL,
    TOKEN_PROGRAM_ID
  )

  //pda
  const [mintProgramSigner, mintNonce] = PublicKey.findProgramAddressSync(
    [assetMint.publicKey.toBuffer(), dex.publicKey.toBuffer()],
    program.programId
  )

  //vault
  const assetVault = await createTokenAccount(assetMint.publicKey, mintProgramSigner)

  await program.methods
    .addAsset(
      BTC_SYMBOL,
      BTC_MINT_DECIMAL,
      mintNonce,
      BTC_ORACLE_SOURCE,
      BORROW_FEE_RATE,
      ADD_LIQUIDITY_FEE_RATE,
      REMOVE_LIQUIDITY_FEE_RATE,
      SWAP_FEE_RATE,
      TARGET_WEIGHT
    )
    .accounts({
      dex: dex.publicKey,
      mint: assetMint.publicKey,
      oracle: btcOracle.publicKey,
      vault: assetVault,
      programSigner: mintProgramSigner,
      authority: authority.publicKey,
    })
    .signers([authority])
    .rpc()

  //Add SOL asset
  const solOracle = await createMockOracle(authority, SOL_ORACLE_PRICE, SOL_ORACLE_PRICE_EXPO)

  const [solProgramSigner, solNonce] = await PublicKey.findProgramAddress(
    [TokenInstructions.WRAPPED_SOL_MINT.toBuffer(), dex.publicKey.toBuffer()],
    program.programId
  )

  //vault
  const solVault = await createTokenAccount(TokenInstructions.WRAPPED_SOL_MINT, solProgramSigner)

  await program.methods
    .addAsset(
      SOL_SYMBOL,
      SOL_MINT_DECIMAL,
      solNonce,
      SOL_ORACLE_SOURCE,
      BORROW_FEE_RATE,
      ADD_LIQUIDITY_FEE_RATE,
      REMOVE_LIQUIDITY_FEE_RATE,
      SWAP_FEE_RATE,
      TARGET_WEIGHT
    )
    .accounts({
      dex: dex.publicKey,
      mint: TokenInstructions.WRAPPED_SOL_MINT,
      oracle: solOracle.publicKey,
      vault: solVault,
      programSigner: solProgramSigner,
      authority: authority.publicKey,
    })
    .signers([authority])
    .rpc()

  //add market
  await program.methods
    .addMarket(
      MARKET_SYMBOL,
      new BN(MINIMUM_COLLATERAL),
      new BN(CHARGE_BORROW_FEE_INTERVAL),
      OPEN_FEE_RATE,
      CLOSE_FEE_RATE,
      LIQUIDATE_FEE_RATE,
      MAX_LEVERAGE,
      DECIMALS,
      BTC_ORACLE_SOURCE,
      ASSET_INDEX,
      SIGNIFICANT_DECIMALS
    )
    .accounts({
      dex: dex.publicKey,
      orderBook: orderBook.publicKey,
      orderPoolEntryPage: orderPoolEntryPage.publicKey,
      authority: authority.publicKey,
      oracle: btcOracle.publicKey,
    })
    .preInstructions([
      await createAccountInstruction(orderBook, 128 * 1024),
      await createAccountInstruction(orderPoolEntryPage, 128 * 1024),
    ])
    .signers([authority, orderBook, orderPoolEntryPage])
    .rpc()

  //init price feed
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

  return {
    dex,
    assetMint,
    assetVault,
    solVault,
    mintProgramSigner,
    mintNonce,
    solProgramSigner,
    solNonce,
    authority,
    eventQueue,
    userListEntryPage,
    orderBook,
    orderPoolEntryPage,
    MOCK_ORACLE_PRICE: BTC_ORACLE_PRICE,
    ADD_LIQUIDITY_FEE_RATE,
    priceFeed,
  }
}
