import { Keypair, PublicKey, AccountMeta } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token'
import { createTokenAccount } from './utils/createTokenAccount'
import { BN } from '@project-serum/anchor'
import { createDexFull } from './utils/createDexFull'
import { getOracleAccounts } from './utils/getOracleAccounts'

describe('Test Remove Liquidity', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_AMOUNT = 10_000_000_000 //10 BTC
  const DEPOSIT_AMOUNT = 1_000_000_000 //1 BTC
  const WITHDRAW_VLP_AMOUNT = 19800_000_000 // 19800 vlp
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

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()

    await airdrop(provider, alice.publicKey, 10000000000)
    ;({ dex, assetMint, assetVault, programSigner, vlpMint, vlpMintAuthority } = await createDexFull(authority))

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

    //add liquidity
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
  })

  it('should remove liquidity success', async () => {
    await program.methods
      .removeLiquidity(new BN(WITHDRAW_VLP_AMOUNT))
      .accounts({
        dex: dex.publicKey,
        mint: assetMint.publicKey,
        vault: assetVault,
        programSigner: programSigner,
        userMintAcc: aliceAssetToken,
        vlpMint: vlpMint,
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
      liquidityAmount: expect.toBNEqual(0),
    })

    expect(aliceVlpTokenAccount).toMatchObject({
      amount: '0',
    })

    console.log(999, aliceAssetTokenAccount, MINT_AMOUNT)
    expect(aliceAssetTokenAccount).toMatchObject({
      amount: (MINT_AMOUNT - 19_900_000).toString(), //fee:{add_liquidity:0.01,remove_liquidity:0.0099}
    })
  })
})
