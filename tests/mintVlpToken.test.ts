import { Keypair, PublicKey } from '@solana/web3.js'
import { createDex } from './utils/createDex'
import { airdrop, getProviderAndProgram } from './utils/getProvider'
import { TOKEN_PROGRAM_ID } from '@solana/spl-token'
import { createTokenAccount } from './utils/createTokenAccount'
import { BN } from '@project-serum/anchor'

describe('Test VLP TOKEN', () => {
  const { program, provider } = getProviderAndProgram()
  const MINT_VLP_AMOUNT = 100
  let dex: Keypair
  let authority: Keypair
  let alice: Keypair
  let vlpMint: PublicKey
  let vlpMintAuthority: PublicKey

  beforeEach(async () => {
    authority = Keypair.generate()
    alice = Keypair.generate()
    await airdrop(provider, alice.publicKey, 10000000000)
    ;({ dex, vlpMint, vlpMintAuthority } = await createDex(authority))
  })

  it('should mint vlp token success', async () => {
    const aliceAta = await createTokenAccount(vlpMint, alice.publicKey)
    await program.methods
      .mintVlpToken(new BN(100))
      .accounts({
        dex: dex.publicKey,
        vlpMint: vlpMint,
        vlpMintAuthority: vlpMintAuthority,
        userTokenAccount: aliceAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        authority: alice.publicKey,
      })
      .signers([alice])
      .rpc()

    const aliceVlpTokenAccount = await (await program.provider.connection.getTokenAccountBalance(aliceAta)).value

    console.log(aliceVlpTokenAccount)
    expect(aliceVlpTokenAccount).toMatchObject({
      amount: MINT_VLP_AMOUNT.toString(),
    })
  })
})
