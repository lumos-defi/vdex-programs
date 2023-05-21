import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccount } from './utils/createAccountInstruction'
import { getProviderAndProgram, airdrop } from './utils/getProvider'
import { createMint, createMintWithKeypair } from './utils/createMint'
import { TokenInstructions } from '@project-serum/serum'
import { createTokenAccount } from './utils/createTokenAccount'

describe('Init Dex', () => {
  const { program, provider } = getProviderAndProgram()
  const VLP_DECIMALS = 6
  const USDC_MINT_DECIMALS = 6

  let dex: Keypair
  let authority: Keypair
  let eventQueue: Keypair
  let matchQueue: Keypair
  let diOption: Keypair
  let priceFeed: Keypair

  let usdcMint: PublicKey
  let rewardMint: PublicKey

  let vdxMint: Keypair
  let vdxVault: PublicKey
  let vdxProgramSigner: PublicKey
  let vdxNonce: number

  beforeAll(async () => {
    dex = Keypair.generate()
    eventQueue = Keypair.generate()
    matchQueue = Keypair.generate()
    diOption = Keypair.generate()
    priceFeed = Keypair.generate()

    authority = Keypair.generate()
    await airdrop(provider, authority.publicKey, 100_000_000_000)

    vdxMint = Keypair.generate()
    ;[vdxProgramSigner, vdxNonce] = PublicKey.findProgramAddressSync(
      [vdxMint.publicKey.toBuffer(), dex.publicKey.toBuffer()],
      program.programId
    )

    await createMintWithKeypair(vdxMint, vdxProgramSigner, authority.publicKey, 6)
    vdxVault = await createTokenAccount(vdxMint.publicKey, vdxProgramSigner)

    usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)

    rewardMint = TokenInstructions.WRAPPED_SOL_MINT
  })

  it('should init dex account successfully', async () => {
    await createAccount(eventQueue, 128 * 1024)
    await createAccount(matchQueue, 128 * 1024)
    await createAccount(diOption, 128 * 1024)

    await program.methods
      .initDex(vdxNonce, 30)
      .accounts({
        dex: dex.publicKey,
        usdcMint,
        authority: authority.publicKey,
        eventQueue: eventQueue.publicKey,
        matchQueue: matchQueue.publicKey,
        vdxProgramSigner,
        vdxMint: vdxMint.publicKey,
        vdxVault,
        rewardMint,
        diOption: diOption.publicKey,
        priceFeed: priceFeed.publicKey,
      })
      .preInstructions([
        await program.account.dex.createInstruction(dex),
        await program.account.priceFeed.createInstruction(priceFeed),
      ])
      .signers([authority, dex, priceFeed])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo).toMatchObject({
      magic: expect.toBNEqual(0x6666),
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      usdcMint: usdcMint,
      assets: expect.arrayContaining([
        expect.objectContaining({
          valid: false,
          decimals: 0,
          nonce: 0,
        }),
      ]),
      markets: expect.arrayContaining([
        expect.objectContaining({
          valid: false,
          decimals: 0,
        }),
      ]),
    })

    expect(dexInfo.vlpPool).toMatchObject({
      mint: PublicKey.default,
      vault: PublicKey.default,
      programSigner: PublicKey.default,
      rewardMint: rewardMint,
      rewardTotal: expect.toBNEqual(0),
      stakedTotal: expect.toBNEqual(0),
      accumulateRewardPerShare: expect.toBNEqual(0),
      rewardAssetIndex: 0xff,
      decimals: VLP_DECIMALS,
      nonce: 255,
    })
  })
})
