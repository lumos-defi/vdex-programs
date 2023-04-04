import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { getOracleAccounts } from './utils/getOracleAccounts'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'
import { createUserState } from './utils/createUserState'
import { TokenInstructions } from '@project-serum/serum'
import { createWSOLAccount } from './utils/createTokenAccount'

describe('Test Add Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_AMOUNT = 10_000_000_000 //10 SOL
  const DEPOSIT_AMOUNT = 1_000_000_000 //1 SOL
  let oracleAccounts = new Array<AccountMeta>()
  let dex: Keypair
  let authority: Keypair

  let solVault: PublicKey

  let alice: Keypair
  let userState: PublicKey
  let eventQueue: Keypair
  let priceFeed: Keypair

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()

    await airdrop(provider, alice.publicKey, MINT_AMOUNT)
    ;({ dex, solVault, eventQueue, priceFeed } = await createDexFull(authority))

    oracleAccounts = await getOracleAccounts(dex.publicKey)
    userState = await createUserState(alice, dex)
  })

  it('should add SOL succeed', async () => {
    // let aliceBalance = await program.provider.connection.getBalance(alice.publicKey)
    const [aliceWSOLAcc, _] = await createWSOLAccount(alice, DEPOSIT_AMOUNT)

    // aliceBalance = await program.provider.connection.getBalance(alice.publicKey)
    // expect(aliceBalance).toBeLessThan(MINT_AMOUNT - DEPOSIT_AMOUNT - accountLamports)

    await program.methods
      .addLiquidity(new BN(DEPOSIT_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: TokenInstructions.WRAPPED_SOL_MINT,
        vault: solVault,
        userMintAcc: aliceWSOLAcc,
        userState: userState,
        eventQueue: eventQueue.publicKey,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceFeed: priceFeed.publicKey,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo.assets[1]).toMatchObject({
      valid: true,
      symbol: Buffer.from('SOL\0\0\0\0\0\0\0\0\0\0\0\0\0'),
      liquidityAmount: expect.toBNEqual(DEPOSIT_AMOUNT - 10_000_000), //fee amount:0.01
    })
  })
})
