import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { getOracleAccounts } from './utils/getOracleAccounts'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'
import { createUserState } from './utils/createUserState'
import { TokenInstructions } from '@project-serum/serum'
import { createWSOLAccount } from './utils/createTokenAccount'

describe('Test Add Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const BTC_MINT_AMOUNT = 10_000_000_000
  const BTC_DEPOSIT_AMOUNT = 1_000_000_000

  const SOL_MINT_AMOUNT = 2000_000_000_000
  const SOL_DEPOSIT_AMOUNT = 1000_000_000_000
  let oracleAccounts = new Array<AccountMeta>()
  let dex: Keypair
  let authority: Keypair

  let btcMint: Token
  let solVault: PublicKey
  let btcVault: PublicKey

  let alice: Keypair
  let aliceBTCAccount: PublicKey
  let userState: PublicKey
  let eventQueue: Keypair
  let priceFeed: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()

    await airdrop(provider, alice.publicKey, SOL_MINT_AMOUNT)
    ;({
      dex,
      assetMint: btcMint,
      assetVault: btcVault,
      solVault,
      eventQueue,
      priceFeed,
    } = await createDexFull(authority))

    // //create alice btc associatedTokenAccount
    aliceBTCAccount = await btcMint.createAssociatedTokenAccount(alice.publicKey)

    //mint asset to alice
    await btcMint.mintTo(
      aliceBTCAccount,
      authority.publicKey, //mint authority
      [authority],
      BTC_MINT_AMOUNT * 2
    )
    oracleAccounts = await getOracleAccounts(dex.publicKey)
    userState = await createUserState(alice, dex)
  })

  it('should add liquidity fail, if SOL is not added first', async () => {
    // No previous fee need to be swapped into SOL
    await program.methods
      .addLiquidity(new BN(BTC_DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: btcMint.publicKey,
        vault: btcVault,
        userMintAcc: aliceBTCAccount,
        userState: userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceFeed: priceFeed.publicKey,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    // Fail this one, need SOL to swap BTC fee into SOL
    await expect(async () => {
      await program.methods
        .addLiquidity(new BN(BTC_DEPOSIT_AMOUNT))
        .accounts({
          dex: dex.publicKey,
          mint: btcMint.publicKey,
          vault: btcVault,
          userMintAcc: aliceBTCAccount,
          userState: userState,
          eventQueue: eventQueue.publicKey,
          authority: alice.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          priceFeed: priceFeed.publicKey,
        })
        .remainingAccounts(oracleAccounts)
        .signers([alice])
        .rpc()
    }).rejects.toThrow()

    const [aliceWSOLAcc, _] = await createWSOLAccount(alice, SOL_DEPOSIT_AMOUNT)
    await program.methods
      .addLiquidity(new BN(SOL_DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: TokenInstructions.WRAPPED_SOL_MINT,
        vault: solVault,
        userMintAcc: aliceWSOLAcc,
        userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceFeed: priceFeed.publicKey,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()
  })

  it('should add liquidity succeed, if SOL is added first', async () => {
    // Add some SOL
    const [aliceWSOLAcc, _] = await createWSOLAccount(alice, SOL_DEPOSIT_AMOUNT)

    await program.methods
      .addLiquidity(new BN(SOL_DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: TokenInstructions.WRAPPED_SOL_MINT,
        vault: solVault,
        userMintAcc: aliceWSOLAcc,
        userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceFeed: priceFeed.publicKey,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    let dexInfo = await program.account.dex.fetch(dex.publicKey)
    expect(dexInfo.assets[1].feeAmount).toBNEqual(10_000_000_000)

    // Add BTC should succeed
    await program.methods
      .addLiquidity(new BN(BTC_DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: btcMint.publicKey,
        vault: btcVault,
        userMintAcc: aliceBTCAccount,
        userState: userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceFeed: priceFeed.publicKey,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    dexInfo = await program.account.dex.fetch(dex.publicKey)
    expect(dexInfo.assets[0].feeAmount).toBNEqual(10_000_000)
    expect(dexInfo.assets[1].feeAmount).toBNEqual(0)
  })
})
