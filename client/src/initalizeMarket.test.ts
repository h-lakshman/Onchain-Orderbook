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
import { MarketStateSchema, InstructionSchema } from "./states";
import {
  createInitializeMintInstruction,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

test("Initialize market", async () => {
  const svm = new LiteSVM();

  const programId = Keypair.generate();
  svm.addProgramFromFile(
    programId.publicKey,
    `${process.env.program_path}`
  );

  const authority = Keypair.generate();
  svm.airdrop(authority.publicKey, BigInt(100 * LAMPORTS_PER_SOL));

  
  // code from here to next comment is for creating dummy mints for testing as lite-svm does not onchain mint
  const baseMintKeypair = Keypair.generate();
  const quoteMintKeypair = Keypair.generate();
  
  const baseMintRent = svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE));
  const createBaseMintAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: baseMintKeypair.publicKey,
    lamports: Number(baseMintRent),
    space: MINT_SIZE,
    programId: TOKEN_PROGRAM_ID,
  });
  
  const initializeBaseMintIx = createInitializeMintInstruction(
    baseMintKeypair.publicKey,
    8,  
    authority.publicKey, 
    null 
  );


  const quoteMintRent = svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE));
  const createQuoteMintAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: quoteMintKeypair.publicKey,
    lamports: Number(quoteMintRent),
    space: MINT_SIZE,
    programId: TOKEN_PROGRAM_ID,
  });
  
  const initializeQuoteMintIx = createInitializeMintInstruction(
    quoteMintKeypair.publicKey,
    6, 
    authority.publicKey, 
    null 
  );

  const createMintsTransaction = new Transaction()
    .add(createBaseMintAccountIx)
    .add(initializeBaseMintIx)
    .add(createQuoteMintAccountIx)
    .add(initializeQuoteMintIx);
  
  createMintsTransaction.feePayer = authority.publicKey;
  createMintsTransaction.recentBlockhash = await svm.latestBlockhash();
  createMintsTransaction.sign(authority, baseMintKeypair, quoteMintKeypair);
  
  const createMintsResult = await svm.sendTransaction(createMintsTransaction);
  if (createMintsResult && typeof createMintsResult === 'object' && 'err' in createMintsResult) {
    console.error("Failed to create mint accounts:", createMintsResult);
    throw new Error("Failed to create mint accounts");
  }
  //end of code for creating dummy mints
  
  
  const baseAsset = baseMintKeypair.publicKey;
  const quoteAsset = quoteMintKeypair.publicKey;

  const [marketAccountPda,marketAccountBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), baseAsset.toBuffer(), quoteAsset.toBuffer()],
    programId.publicKey
  );

  const [marketEventsPda,marketEventsBump] = PublicKey.findProgramAddressSync([Buffer.from("events"), marketAccountPda.toBuffer()], programId.publicKey);

  const [baseVaultPda,baseVaultBump] = PublicKey.findProgramAddressSync([Buffer.from("base_vault"), marketAccountPda.toBuffer()], programId.publicKey);

  const [quoteVaultPda,quoteVaultBump] = PublicKey.findProgramAddressSync([Buffer.from("quote_vault"), marketAccountPda.toBuffer()], programId.publicKey);

  const [feeAccountPda,feeAccountBump] = PublicKey.findProgramAddressSync([Buffer.from("fee_account"), marketAccountPda.toBuffer()], programId.publicKey);

  const minOrderSize = new BN(1_000_000);
  const tickSize = new BN(1_000);

  const splTokenProgramId = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

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

  const initializeMarketIx = new TransactionInstruction({
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
        pubkey: baseVaultPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: quoteVaultPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: marketEventsPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: feeAccountPda,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: SystemProgram.programId,
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: splTokenProgramId, 
        isSigner: false,
        isWritable: false,
      },
      {
        pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
    data: dataBuffer,
  });
    const tx = new Transaction().add(initializeMarketIx);
  tx.feePayer = authority.publicKey;
  tx.recentBlockhash = await svm.latestBlockhash();
  tx.sign(authority);

  const result = await svm.sendTransaction(tx);
  
  if (result && typeof result === 'object' && 'err' in result) {
    console.error("Transaction failed:", result);
    throw new Error("Transaction failed");
  }
  
  console.log("Transaction successful!");

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
  expect(marketState.isInitialized).toBeTrue();

  console.log("Market initialized successfully!");
  console.log("Market State:", {
    authority: marketState.authority.toBase58(),
    baseMint: marketState.baseMint.toBase58(),
    quoteMint: marketState.quoteMint.toBase58(),
    minOrderSize: marketState.minOrderSize.toString(),
    tickSize: marketState.tickSize.toString(),
    nextOrderId: marketState.nextOrderId.toString(),
    isInitialized: marketState.isInitialized,
  });
  
});
