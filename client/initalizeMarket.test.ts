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
import BN from "bn.js";
import { MarketStateSchema, InstructionSchema } from "./types";


test("Initialize market", async () => {
  const svm = new LiteSVM();

  const programId = Keypair.generate();
  svm.addProgramFromFile(
    programId.publicKey,
    `${process.env.program_path}`
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

  const minOrderSize = new BN(1_000_000);
  const tickSize = new BN(1_000);

  const dataBuffer = Buffer.alloc(17);
  InstructionSchema.encode(
    {
      InitializeMarket: {
        min_order_size: minOrderSize,
        tick_size: tickSize,
      },
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

  const accountData = Buffer.from(marketAccount!.data);
  const marketState = MarketStateSchema.decode(accountData);

  expect(marketState.authority.equals(authority.publicKey)).toBeTrue();
  expect(marketState.baseMint.equals(baseAsset)).toBeTrue();
  expect(marketState.quoteMint.equals(quoteAsset)).toBeTrue();
  expect(marketState.minOrderSize.eq(minOrderSize)).toBeTrue();
  expect(marketState.tickSize.eq(tickSize)).toBeTrue();
  expect(marketState.nextOrderId.eq(new BN(1))).toBeTrue();
  expect(marketState.isActive).toBeTrue();

  console.log("Market initialized successfully!");
  console.log("Market State:", {
    authority: marketState.authority.toBase58(),
    baseMint: marketState.baseMint.toBase58(),
    quoteMint: marketState.quoteMint.toBase58(),
    minOrderSize: marketState.minOrderSize.toString(),
    tickSize: marketState.tickSize.toString(),
    nextOrderId: marketState.nextOrderId.toString(),
    isActive: marketState.isActive,
  });
});
