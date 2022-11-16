import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { getOracleAccounts } from './utils/getOracleAccounts'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'
import { createUserState } from './utils/createUserState'
import { createAccountInstruction } from './utils/createAccountInstruction'

describe('Test Add Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_AMOUNT = 10_000_000_000 //10 BTC
  const DEPOSIT_AMOUNT = 1_000_000_000 //1 BTC
  let oracleAccounts = new Array<AccountMeta>()
  let dex: Keypair
  let authority: Keypair

  let assetMint: Token
  let assetVault: PublicKey
  let programSigner: PublicKey

  let alice: Keypair
  let aliceAssetToken: PublicKey
  let userState: PublicKey
  let eventQueue: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()

    await airdrop(provider, alice.publicKey, 10000000000)
    ;({ dex, assetMint, assetVault, programSigner, eventQueue } = await createDexFull(authority))

    //create alice asset associatedTokenAccount
    aliceAssetToken = await assetMint.createAssociatedTokenAccount(alice.publicKey)

    //mint asset to alice
    await assetMint.mintTo(
      aliceAssetToken,
      authority.publicKey, //mint authority
      [authority],
      MINT_AMOUNT
    )
    oracleAccounts = await getOracleAccounts(dex.publicKey)

    //create userState
    userState = await createUserState(alice, dex)
  })

  it('should add liquidity success', async () => {
    await program.methods
      .addLiquidity(new BN(DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: assetMint.publicKey,
        vault: assetVault,
        programSigner: programSigner,
        userMintAcc: aliceAssetToken,
        userState: userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    const aliceAssetTokenAccount = await (
      await program.provider.connection.getTokenAccountBalance(aliceAssetToken)
    ).value

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo.assets[0]).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC\0\0\0\0\0\0\0\0\0\0\0\0\0'),
      liquidityAmount: expect.toBNEqual(DEPOSIT_AMOUNT - 10_000_000), //fee amount:0.01
    })

    expect(aliceAssetTokenAccount).toMatchObject({
      amount: (MINT_AMOUNT - DEPOSIT_AMOUNT).toString(),
    })
  })
})
