import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SolnameCredit } from "../target/types/solname_credit";
import { assert } from "chai";
import { createMint, getAssociatedTokenAddress, createAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";

describe("solname-credit p2p tests", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SolnameCredit as Program<SolnameCredit>;

  const lender = anchor.web3.Keypair.generate();
  let usdcMint: anchor.web3.PublicKey;
  let lenderUsdc: anchor.web3.PublicKey;

  // P2P Offer state
  const nonce = new anchor.BN(Date.now());
  const [offerAccountPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("offer"), lender.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
    program.programId
  );

  const [offerVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), offerAccountPda.toBuffer()],
    program.programId
  );

  before(async () => {
      // Setup lender
      await provider.connection.confirmTransaction(
          await provider.connection.requestAirdrop(lender.publicKey, 10 * anchor.web3.LAMPORTS_PER_SOL),
          "confirmed"
      );

      // Create Mint (Mock)
      usdcMint = await createMint(
          provider.connection,
          lender,
          lender.publicKey,
          null,
          6
      );

      // Mint USDC to lender
      lenderUsdc = await createAssociatedTokenAccount(
          provider.connection,
          lender,
          usdcMint,
          lender.publicKey
      );
      await mintTo(
          provider.connection,
          lender,
          usdcMint,
          lenderUsdc,
          lender,
          1000000000 // 1000 USDC
      );
  });

  it("Creates a P2P offer", async () => {
    const principal = new anchor.BN(100000000); // 100 USDC
    const aprBps = 1000; // 10%
    const durationSeconds = new anchor.BN(86400 * 30); // 30 days
    const offerExpiry = new anchor.BN(Math.floor(Date.now() / 1000) + 86400 * 7); // Expires in 7 days

    await program.methods
      .createOffer(nonce, principal, aprBps, durationSeconds, offerExpiry)
      .accounts({
        lender: lender.publicKey,
        offerAccount: offerAccountPda,
        offerVault: offerVaultPda,
        lenderUsdc: lenderUsdc,
        usdcMint: usdcMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([lender])
      .rpc();

    const offer = await program.account.offerAccount.fetch(offerAccountPda);
    assert.ok(offer.lender.equals(lender.publicKey));
    assert.ok(offer.principal.eq(principal));
    assert.ok(offer.isActive);
  });

  it("Cancels a P2P offer", async () => {
    await program.methods
      .cancelOffer()
      .accounts({
        lender: lender.publicKey,
        offerAccount: offerAccountPda,
        offerVault: offerVaultPda,
        lenderUsdc: lenderUsdc,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([lender])
      .rpc();

    // Account should be closed
    try {
        await program.account.offerAccount.fetch(offerAccountPda);
        assert.fail("Account should be closed");
    } catch (e) {
        assert.ok(e.message.includes("Account does not exist"));
    }
  });
});
