import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccountInstruction } from './utils/createAccountInstruction'
import { getProviderAndProgram, airdrop } from './utils/getProvider'
// import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { createMint } from './utils/createMint'

describe('Init Dex', () => {
  const { program, provider } = getProviderAndProgram()
  const VLP_DECIMALS = 6
  const USDC_MINT_DECIMALS = 6

  let dex: Keypair
  let authority: Keypair
  let eventQueue: Keypair
  let matchQueue: Keypair
  let userListEntryPage: Keypair

  let usdcMint: PublicKey
  let vlpMint: PublicKey
  let vlpMintAuthority: PublicKey
  let vlpMintNonce: number

  beforeAll(async () => {
    dex = Keypair.generate()
    eventQueue = Keypair.generate()
    matchQueue = Keypair.generate()
    userListEntryPage = Keypair.generate()
    authority = Keypair.generate()

    await airdrop(provider, authority.publicKey, 100_000_000_000)
    usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)

    //gen vlp mint with seeds
    ;[vlpMint] = await PublicKey.findProgramAddress([dex.publicKey.toBuffer(), Buffer.from('vlp')], program.programId)
    ;[vlpMintAuthority, vlpMintNonce] = await PublicKey.findProgramAddress(
      [dex.publicKey.toBuffer(), vlpMint.toBuffer()],
      program.programId
    )
  })

  it('should init dex account successfully', async () => {
    await program.methods
      .initDex(VLP_DECIMALS, vlpMintNonce)
      .accounts({
        dex: dex.publicKey,
        usdcMint: usdcMint,
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

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo).toMatchObject({
      magic: expect.toBNEqual(0x6666),
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
      usdcMint: usdcMint,
      vlpMint: vlpMint,
      vlpMintNonce: vlpMintNonce,
      vlpMintAuthority: vlpMintAuthority,
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
  })
})
