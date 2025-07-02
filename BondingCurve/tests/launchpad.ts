import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Launchpad } from "../target/types/launchpad";
import {
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
  SystemProgram,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";
import { getAssociatedTokenAddressSync } from "@solana/spl-token";

describe("Launchpad", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const user = provider.wallet;
  const program = anchor.workspace.Launchpad as Program<Launchpad>;

  it("should create a new pool", async () => {
    // WSOL mint address
    const WSOL_MINT = new PublicKey(
      "So11111111111111111111111111111111111111112"
    );

    const pool = Keypair.generate();
    const tokenMint = Keypair.generate();
    const memeVault = Keypair.generate();
    const memeMint = Keypair.generate();
    const targetConfig = Keypair.generate();
    // Derive the associated token account for WSOL (quoteVault)
    const quoteVault = getAssociatedTokenAddressSync(
      WSOL_MINT,
      pool.publicKey, // Assuming pool is the owner of the quoteVault
      true // Allow owner off-curve for PDAs
    );

    const targetAmount = new BN(1_000_000_000); // 1 billion base units

    const tx = await program.methods
      .newPool(targetAmount)
      .accountsPartial({
        sender: user.publicKey,
        feeQuoteVault: quoteVault,
        memeMint: memeMint.publicKey,
        quoteMint: WSOL_MINT,
        pool: pool.publicKey,
        targetConfig: targetConfig.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([pool, tokenMint, memeVault, memeMint, targetConfig])
      .rpc();
    console.log("Pool created:", tx);
  });
});
