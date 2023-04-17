import { TokenInstructions } from '@project-serum/serum'
import { Keypair, PublicKey } from '@solana/web3.js'
import { createAccount } from './createAccountInstruction'
import { createMint, createMintWithKeypair } from './createMint'
import { createTokenAccount } from './createTokenAccount'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createDex(authority: Keypair) {
  const { program, provider } = getProviderAndProgram()
  const dex = Keypair.generate()
  const eventQueue = Keypair.generate()
  const matchQueue = Keypair.generate()
  const userListEntryPage = Keypair.generate()
  const diOption = Keypair.generate()
  const priceFeed = Keypair.generate()
  const rewardMint = TokenInstructions.WRAPPED_SOL_MINT
  const USDC_MINT_DECIMALS = 6

  await airdrop(provider, authority.publicKey, 10000000000)

  const vdxMint = Keypair.generate()

  const [vdxProgramSigner, vdxNonce] = PublicKey.findProgramAddressSync(
    [vdxMint.publicKey.toBuffer(), dex.publicKey.toBuffer()],
    program.programId
  )

  await createMintWithKeypair(vdxMint, vdxProgramSigner, authority.publicKey, 6)
  const vdxVault = await createTokenAccount(vdxMint.publicKey, vdxProgramSigner)

  const usdcMint = await createMint(authority.publicKey, USDC_MINT_DECIMALS)

  await createAccount(eventQueue, 128 * 1024)
  await createAccount(matchQueue, 128 * 1024)
  await createAccount(userListEntryPage, 128 * 1024)
  await createAccount(diOption, 128 * 1024)

  await program.methods
    .initDex(vdxNonce, 30)
    .accounts({
      dex: dex.publicKey,
      usdcMint,
      authority: authority.publicKey,
      eventQueue: eventQueue.publicKey,
      matchQueue: matchQueue.publicKey,
      userListEntryPage: userListEntryPage.publicKey,
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

  return { dex }
}
