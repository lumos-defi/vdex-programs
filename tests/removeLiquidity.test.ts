import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'
import { getOracleAccounts } from './utils/getOracleAccounts'
import { createUserState } from './utils/createUserState'
import { TokenInstructions } from '@project-serum/serum'
import { createWSOLAccount } from './utils/createTokenAccount'

describe('Test Remove Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_AMOUNT = 10_000_000_000 //10 BTC
  const DEPOSIT_AMOUNT = 1_000_000_000 //1 BTC

  const ADD_FEE = 10_000_000
  const REMOVE_FEE = 10_000_000 - 100_000

  const WITHDRAW_VLP_AMOUNT = 19800_000_000 // 19800 vlp vlp_decimals=6
  const SOL_MINT_AMOUNT = 2000_000_000_000
  const SOL_DEPOSIT_AMOUNT = 1000_000_000_000

  let oracleAccounts = new Array<AccountMeta>()
  let dex: Keypair
  let authority: Keypair

  let assetMint: Token
  let assetVault: PublicKey
  let solVault: PublicKey
  let mintProgramSigner: PublicKey

  let alice: Keypair
  let bob: Keypair
  let aliceMintAcc: PublicKey
  let aliceUserState: PublicKey
  let bobUserState: PublicKey
  let eventQueue: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()
    bob = Keypair.generate()

    await airdrop(provider, bob.publicKey, SOL_MINT_AMOUNT)
    ;({ dex, assetMint, assetVault, solVault, mintProgramSigner, eventQueue } = await createDexFull(authority))

    //create alice asset associatedTokenAccount
    aliceMintAcc = await assetMint.createAssociatedTokenAccount(alice.publicKey)

    //mint asset to alice
    await assetMint.mintTo(
      aliceMintAcc,
      authority.publicKey, //mint authority
      [authority],
      MINT_AMOUNT
    )

    oracleAccounts = await getOracleAccounts(dex.publicKey)

    //create userState
    aliceUserState = await createUserState(alice, dex)
    bobUserState = await createUserState(bob, dex)

    // Add some SOL
    const [bobWSOLAcc, _] = await createWSOLAccount(bob, SOL_DEPOSIT_AMOUNT)

    await program.methods
      .addLiquidity(new BN(SOL_DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: TokenInstructions.WRAPPED_SOL_MINT,
        vault: solVault,
        userMintAcc: bobWSOLAcc,
        userState: bobUserState,
        eventQueue: eventQueue.publicKey,
        authority: bob.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleAccounts)
      .signers([bob])
      .rpc()

    //add liquidity
    await program.methods
      .addLiquidity(new BN(DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: assetMint.publicKey,
        vault: assetVault,
        userMintAcc: aliceMintAcc,
        userState: aliceUserState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()
  })

  it('should remove liquidity success', async () => {
    await program.methods
      .removeLiquidity(new BN(WITHDRAW_VLP_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: assetMint.publicKey,
        vault: assetVault,
        programSigner: mintProgramSigner,
        userMintAcc: aliceMintAcc,
        authority: alice.publicKey,
        userState: aliceUserState,
        eventQueue: eventQueue.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)
    expect(dexInfo.assets[0]).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC\0\0\0\0\0\0\0\0\0\0\0\0\0'),
      liquidityAmount: expect.toBNEqual(ADD_FEE + REMOVE_FEE),
    })
    const aliceAssetTokenAccount = await (await program.provider.connection.getTokenAccountBalance(aliceMintAcc)).value
    expect(aliceAssetTokenAccount).toMatchObject({
      amount: (MINT_AMOUNT - 19_900_000).toString(), //fee:{add_liquidity:0.01,remove_liquidity:0.0099}
    })
  })
})
