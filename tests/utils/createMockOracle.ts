import { BN } from '@project-serum/anchor'
import { Keypair, SystemProgram } from '@solana/web3.js'
import { airdrop, getProviderAndProgram } from './getProvider'

export async function createMockOracle(authority: Keypair, price, expo: number) {
  const { program, provider } = getProviderAndProgram()
  const mockOracle = Keypair.generate()

  await airdrop(provider, authority.publicKey, 10000000000)

  await program.methods
    .initMockOracle(new BN(price), expo)
    .accounts({
      mockOracle: mockOracle.publicKey,
      authority: authority.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .signers([authority, mockOracle])
    .rpc()

  return { mockOracle }
}
