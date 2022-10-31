import { PublicKey, Keypair } from '@solana/web3.js'
import { createDex } from './utils/createDex'
import { createMint } from './utils/createMint'
import { createMockOracle } from './utils/createMockOracle'
import { createTokenAccount } from './utils/createTokenAccount'
import { getProviderAndProgram } from './utils/getProvider'

describe('Add Dex Asset', () => {
  const { program } = getProviderAndProgram()

  const ASSET_SYMBOL = 'BTC'
  const ASSET_MINT_DECIMAL = 9
  const BORROWED_FEE_RATE = 100 // 1-10000, the percentage will be XXXX_RATE / 10000
  const ADD_LIQUIDITY_FEE_RATE = 100
  const REMOVE_LIQUIDITY_FEE_RATE = 100
  const TARGET_WEIGHT = 100 //1-1000, the percentage will be weight / 1000

  const WRONG_NONCE = 123
  const WRONG_MINT = Keypair.generate().publicKey

  const MOCK_ORACLE_PRICE = 2000_000_000_000 //$20000
  const MOCK_ORACLE_PRICE_EXPO = 8
  const ORACLE_SOURCE = 0 // 0:mock,1:pyth

  let authority: Keypair
  let dex: Keypair
  let mockOracle: Keypair
  let programSigner: PublicKey
  let assetMint: PublicKey
  let assetVault: PublicKey
  let wrongAssetVault: PublicKey
  let nonce: number

  beforeEach(async () => {
    authority = Keypair.generate()
    ;({ dex } = await createDex(authority))
    ;({ mockOracle } = await createMockOracle(authority, MOCK_ORACLE_PRICE, MOCK_ORACLE_PRICE_EXPO))

    //mint
    assetMint = await createMint(authority.publicKey, ASSET_MINT_DECIMAL)

    //pda
    ;[programSigner, nonce] = await PublicKey.findProgramAddress(
      [assetMint.toBuffer(), dex.publicKey.toBuffer()],
      program.programId
    )

    //vault
    assetVault = await createTokenAccount(assetMint, programSigner)

    wrongAssetVault = await createTokenAccount(assetMint, Keypair.generate().publicKey)
  })

  it.each`
    symbol    | mint                | vault                    | signerNonce          | expectError
    ${'BTC'}  | ${() => assetMint}  | ${() => assetVault}      | ${() => nonce}       | ${'Duplicate asset'}
    ${'BTC1'} | ${() => assetMint}  | ${() => assetVault}      | ${() => nonce}       | ${'Duplicate asset'}
    ${'BTC'}  | ${() => assetMint}  | ${() => assetVault}      | ${() => WRONG_NONCE} | ${'Invalid program signer'}
    ${'BTC'}  | ${() => assetMint}  | ${() => wrongAssetVault} | ${() => nonce}       | ${'Invalid program signer'}
    ${'BTC'}  | ${() => WRONG_MINT} | ${() => assetVault}      | ${() => nonce}       | ${'Invalid mint'}
  `('should create asset failed', async ({ symbol, mint, vault: vault, signerNonce, expectError }) => {
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
        mint: assetMint,
        oracle: mockOracle.publicKey,
        vault: assetVault,
        programSigner: programSigner,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc()

    await expect(async () => {
      await program.methods
        .addAsset(
          symbol,
          ASSET_MINT_DECIMAL,
          signerNonce(),
          ORACLE_SOURCE,
          BORROWED_FEE_RATE,
          ADD_LIQUIDITY_FEE_RATE,
          REMOVE_LIQUIDITY_FEE_RATE,
          TARGET_WEIGHT
        )
        .accounts({
          dex: dex.publicKey,
          mint: mint(),
          vault: vault(),
          programSigner: programSigner,
          oracle: mockOracle.publicKey,
          authority: authority.publicKey,
        })
        .signers([authority])
        .rpc()
    }).rejects.toThrow(expectError)
  })

  it('should add perp asset successfully', async () => {
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
        mint: assetMint,
        oracle: mockOracle.publicKey,
        vault: assetVault,
        programSigner: programSigner,
        authority: authority.publicKey,
      })
      .signers([authority])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo).toMatchObject({
      magic: expect.toBNEqual(0x6666),
      assetsNumber: 1,
    })

    expect(dexInfo.assets[0]).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC\0\0\0\0\0\0\0\0\0\0\0\0\0'),
      decimals: ASSET_MINT_DECIMAL,
      nonce: nonce,
      mint: assetMint,
      vault: assetVault,
      programSigner: programSigner,
      borrowedFeeRate: BORROWED_FEE_RATE,
      addLiquidityFeeRate: ADD_LIQUIDITY_FEE_RATE,
      removeLiquidityFeeRate: REMOVE_LIQUIDITY_FEE_RATE,
      targetWeight: TARGET_WEIGHT,
    })
  })
})
