import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { createTokenAccount } from './utils/createTokenAccount'
import { getOracleAccounts } from './utils/getOracleAccounts'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'

describe('Test Add Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_AMOUNT = 10_000_000_000 //10 BTC
  const DEPOSIT_AMOUNT = 1_000_000_000 //1 BTC
  let oracleAccounts = new Array<AccountMeta>()
  let dex: Keypair
  let authority: Keypair

  let vlpMint: PublicKey
  let vlpMintAuthority: PublicKey

  let assetMint: Token
  let assetVault: PublicKey
  let programSigner: PublicKey

  let alice: Keypair
  let aliceAssetToken: PublicKey
  let aliceVlpToken: PublicKey
  let MOCK_ORACLE_PRICE: number

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()

    await airdrop(provider, alice.publicKey, 10000000000)
    ;({ dex, assetMint, assetVault, programSigner, vlpMint, vlpMintAuthority, MOCK_ORACLE_PRICE } = await createDexFull(
      authority
    ))

    //create alice asset associatedTokenAccount
    aliceAssetToken = await assetMint.createAssociatedTokenAccount(alice.publicKey)
    aliceVlpToken = await createTokenAccount(vlpMint, alice.publicKey)

    //mint asset to alice
    await assetMint.mintTo(
      aliceAssetToken,
      authority.publicKey, //mint authority
      [authority],
      MINT_AMOUNT
    )
    oracleAccounts = await getOracleAccounts(dex.publicKey)
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
        vlpMint: vlpMint,
        vlpMintAuthority: vlpMintAuthority,
        userVlpAccount: aliceVlpToken,
        authority: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .remainingAccounts(oracleAccounts)
      .signers([alice])
      .rpc()

    const aliceAssetTokenAccount = await (
      await program.provider.connection.getTokenAccountBalance(aliceAssetToken)
    ).value

    const aliceVlpTokenAccount = await (await program.provider.connection.getTokenAccountBalance(aliceVlpToken)).value

    const dexInfo = await program.account.dex.fetch(dex.publicKey)

    expect(dexInfo.assets[0]).toMatchObject({
      valid: true,
      symbol: Buffer.from('BTC\0\0\0\0\0\0\0\0\0\0\0\0\0'),
      liquidityAmount: expect.toBNEqual(DEPOSIT_AMOUNT),
    })

    expect(aliceAssetTokenAccount).toMatchObject({
      amount: (MINT_AMOUNT - DEPOSIT_AMOUNT).toString(),
    })

    console.log(99999, MOCK_ORACLE_PRICE, aliceVlpTokenAccount)
    expect(aliceVlpTokenAccount).toMatchObject({
      amount: '19800000000', //size:1 btc, price:20000,fee:200
    })
  })
})
