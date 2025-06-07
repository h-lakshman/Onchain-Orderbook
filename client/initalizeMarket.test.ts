import { LiteSVM } from "litesvm";
import { expect, test } from "bun:test";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import * as borsh from "@coral-xyz/borsh";

const MarketStateSchema = borsh.struct([
  borsh.publicKey("authority"),
  borsh.publicKey("baseMint"),
  borsh.publicKey("quoteMint"),
  borsh.u64("minOrderSize"),
  borsh.u64("tickSize"),
  borsh.u64("nextOrderId"),
  borsh.bool("isActive"),
]);

const InitializeMarketInstructionSchema = borsh.struct([
  borsh.u8("variant"),
  borsh.u64("min_order_size"),
  borsh.u64("tick_size"),
]);

test("Initialize market", async () => {
  const svm = new LiteSVM();

  const programId = Keypair.generate();
  svm.addProgramFromFile(
    programId.publicKey,
    "../program/target/deploy/program.so"
  );
  const baseAsset = new PublicKey(
    "So11111111111111111111111111111111111111112"
  ); // WSOL
  const quoteAsset = new PublicKey(
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
  ); // USDC
  const authority = Keypair.generate();
  svm.airdrop(authority.publicKey, BigInt(10 * LAMPORTS_PER_SOL));

  const [marketAccountPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), baseAsset.toBuffer(), quoteAsset.toBuffer()],
    programId.publicKey
  );

  const minOrderSize = 1_000_000n;
  const tickSize = 1_000n;

  const dataBuffer = Buffer.alloc(17);
  InitializeMarketInstructionSchema.encode(
    {
      variant: 0,
      min_order_size: minOrderSize,
      tick_size: tickSize,
    },
    dataBuffer
  );

  const ix = new TransactionInstruction({
    programId: programId.publicKey,
    keys: [
      {
        pubkey: authority.publicKey,
        isSigner: true,
        isWritable: true,
      },
      {
        pubkey: marketAccountPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: baseAsset,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: quoteAsset,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
    ],
    data: dataBuffer,
  });

  const tx = new Transaction().add(ix);
  tx.feePayer = authority.publicKey;
  tx.recentBlockhash = await svm.latestBlockhash();
  tx.sign(authority);

  await svm.sendTransaction(tx);

  const marketAccount = svm.getAccount(marketAccountPda);
  expect(marketAccount).toBeDefined();

  const marketState = MarketStateSchema.decode(marketAccount!.data);

  expect(marketState.authority.equals(authority.publicKey)).toBeTrue();
  expect(marketState.baseMint.equals(baseAsset)).toBeTrue();
  expect(marketState.quoteMint.equals(quoteAsset)).toBeTrue();
  expect(marketState.minOrderSize).toBe(minOrderSize);
  expect(marketState.tickSize).toBe(tickSize);
  expect(marketState.nextOrderId).toBe(1n);
  expect(marketState.isActive).toBeTrue();

  console.log("✅ Market initialized successfully!");
  console.log("Market State:", {
    ...marketState,
    authority: marketState.authority.toBase58(),
    baseMint: marketState.baseMint.toBase58(),
    quoteMint: marketState.quoteMint.toBase58(),
  });
});
