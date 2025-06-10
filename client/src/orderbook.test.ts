import {
  createInitializeMintInstruction,
  createMintToInstruction,
  createInitializeAccountInstruction,
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import { expect, test, beforeAll } from "bun:test";
import { LiteSVM } from "litesvm";
import { InstructionSchema, MarketStateSchema, UserBalance } from "./states";
import { BN } from "bn.js";

let testEnv: {
  svm: LiteSVM;
  programId: Keypair;
  authority: Keypair;
  user: Keypair;
  baseAsset: PublicKey;
  quoteAsset: PublicKey;
  userQuoteTokenAccount: PublicKey;
  userBaseTokenAccount: PublicKey;
  marketAccountPda: PublicKey;
  bidsPda: PublicKey;
  asksPda: PublicKey;
  marketEventsPda: PublicKey;
  baseVaultPda: PublicKey;
  quoteVaultPda: PublicKey;
  feeAccountPda: PublicKey;
  userBalancePda: PublicKey;
};

beforeAll(async () => {
  const svm = new LiteSVM();

  const programId = Keypair.generate();
  svm.addProgramFromFile(programId.publicKey, process.env.program_path || "");

  const authority = Keypair.generate();
  svm.airdrop(authority.publicKey, BigInt(100 * LAMPORTS_PER_SOL));

  const user = Keypair.generate();
  svm.airdrop(user.publicKey, BigInt(100 * LAMPORTS_PER_SOL));

  // Create SOL and USDC mints
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
    9,
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

  const userQuoteTokenAccount = Keypair.generate();
  const userBaseTokenAccount = Keypair.generate();

  const userQuoteTokenAccountRent = svm.minimumBalanceForRentExemption(
    BigInt(165)
  );
  const createUserQuoteTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: userQuoteTokenAccount.publicKey,
    lamports: Number(userQuoteTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const initializeUserQuoteTokenAccountIx = createInitializeAccountInstruction(
    userQuoteTokenAccount.publicKey,
    quoteMintKeypair.publicKey,
    user.publicKey
  );

  const userBaseTokenAccountRent = svm.minimumBalanceForRentExemption(
    BigInt(165)
  );
  const createUserBaseTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: userBaseTokenAccount.publicKey,
    lamports: Number(userBaseTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const initializeUserBaseTokenAccountIx = createInitializeAccountInstruction(
    userBaseTokenAccount.publicKey,
    baseMintKeypair.publicKey,
    user.publicKey
  );

  // Mint tokens to user accounts
  const mintQuoteToUserIx = createMintToInstruction(
    quoteMintKeypair.publicKey,
    userQuoteTokenAccount.publicKey,
    authority.publicKey,
    1000_000_000
  );

  const mintBaseToUserIx = createMintToInstruction(
    baseMintKeypair.publicKey,
    userBaseTokenAccount.publicKey,
    authority.publicKey,
    10_000_000_000
  );

  const setupTransaction = new Transaction()
    .add(createBaseMintAccountIx)
    .add(initializeBaseMintIx)
    .add(createQuoteMintAccountIx)
    .add(initializeQuoteMintIx)
    .add(createUserQuoteTokenAccountIx)
    .add(initializeUserQuoteTokenAccountIx)
    .add(createUserBaseTokenAccountIx)
    .add(initializeUserBaseTokenAccountIx)
    .add(mintQuoteToUserIx)
    .add(mintBaseToUserIx);

  setupTransaction.feePayer = authority.publicKey;
  setupTransaction.recentBlockhash = await svm.latestBlockhash();
  setupTransaction.sign(
    authority,
    baseMintKeypair,
    quoteMintKeypair,
    userQuoteTokenAccount,
    userBaseTokenAccount
  );

  const setupResult = await svm.sendTransaction(setupTransaction);
  if (setupResult && typeof setupResult === "object" && "err" in setupResult) {
    throw new Error("Failed to setup mints and user token accounts");
  }

  const baseAsset = baseMintKeypair.publicKey;
  const quoteAsset = quoteMintKeypair.publicKey;

  const [marketAccountPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), baseAsset.toBuffer(), quoteAsset.toBuffer()],
    programId.publicKey
  );

  const [bidsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("bids"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [asksPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("asks"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [marketEventsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("events"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [baseVaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("base_vault"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [quoteVaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("quote_vault"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [feeAccountPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("fee_account"), marketAccountPda.toBuffer()],
    programId.publicKey
  );

  const [userBalancePda] = PublicKey.findProgramAddressSync(
    [Buffer.from("balance"), user.publicKey.toBuffer()],
    programId.publicKey
  );

  testEnv = {
    svm,
    programId,
    authority,
    user,
    baseAsset,
    quoteAsset,
    userQuoteTokenAccount: userQuoteTokenAccount.publicKey,
    userBaseTokenAccount: userBaseTokenAccount.publicKey,
    marketAccountPda,
    bidsPda,
    asksPda,
    marketEventsPda,
    baseVaultPda,
    quoteVaultPda,
    feeAccountPda,
    userBalancePda,
  };

  console.log(" Test environment setup complete:");
  console.log("   SOL/USDC Market");
  console.log(" User has 1000 USDC and 10 SOL");
});

test("Initialize Market", async () => {
  const {
    svm,
    programId,
    authority,
    baseAsset,
    quoteAsset,
    marketAccountPda,
    bidsPda,
    asksPda,
    marketEventsPda,
    baseVaultPda,
    quoteVaultPda,
    feeAccountPda,
  } = testEnv;

  const minOrderSize = new BN(1_000_000);
  const tickSize = new BN(1_000);

  const marketDataBuffer = Buffer.alloc(17);
  InstructionSchema.encode(
    {
      InitializeMarket: {
        min_order_size: minOrderSize,
        tick_size: tickSize,
      },
    },
    marketDataBuffer
  );

  const initializeMarketIx = new TransactionInstruction({
    programId: programId.publicKey,
    keys: [
      { pubkey: authority.publicKey, isSigner: true, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: baseAsset, isSigner: false, isWritable: false },
      { pubkey: quoteAsset, isSigner: false, isWritable: false },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      { pubkey: feeAccountPda, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
    data: marketDataBuffer,
  });

  const marketTx = new Transaction().add(initializeMarketIx);
  marketTx.feePayer = authority.publicKey;
  marketTx.recentBlockhash = await svm.latestBlockhash();
  marketTx.sign(authority);

  const marketResult = await svm.sendTransaction(marketTx);

  if (
    marketResult &&
    typeof marketResult === "object" &&
    "err" in marketResult
  ) {
    console.error("Market initialization failed:", marketResult);
    if (marketResult.meta && marketResult.meta().logs) {
      console.log("Program logs:");
      marketResult
        .meta()
        .logs()
        .forEach((log: string, index: number) => {
          console.log(`${index + 1}: ${log}`);
        });
    }
    throw new Error("Market initialization failed");
  }

  const marketAccount = svm.getAccount(marketAccountPda);
  expect(marketAccount).toBeDefined();

  const accountData = Buffer.from(marketAccount!.data);
  const marketState = MarketStateSchema.decode(accountData);
  expect(marketState.authority.equals(authority.publicKey)).toBeTrue();
  expect(marketState.baseMint.equals(baseAsset)).toBeTrue();
  expect(marketState.quoteMint.equals(quoteAsset)).toBeTrue();
  expect(marketState.minOrderSize.eq(minOrderSize)).toBeTrue();
  expect(marketState.tickSize.eq(tickSize)).toBeTrue();
  expect(marketState.isInitialized).toBeTrue();

  console.log(" Market initialized successfully!");
});

test("Create User Balance Account", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    quoteVaultPda,
    userBalancePda,
    userQuoteTokenAccount,
  } = testEnv;

  // testy by depositing 100 usdc
  const depositAmount = new BN(100_000_000);

  const userBalanceDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      CreateUserBalanceAccount: {
        onramp_quantity: depositAmount,
      },
    },
    userBalanceDataBuffer
  );

  const createUserBalanceIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: userBalanceDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: false },
      { pubkey: userQuoteTokenAccount, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const userBalanceTx = new Transaction().add(createUserBalanceIx);
  userBalanceTx.feePayer = user.publicKey;
  userBalanceTx.recentBlockhash = await svm.latestBlockhash();
  userBalanceTx.sign(user);

  const userBalanceResult = await svm.sendTransaction(userBalanceTx);

  if (
    userBalanceResult &&
    typeof userBalanceResult === "object" &&
    "err" in userBalanceResult
  ) {
    console.error("User balance creation failed:", userBalanceResult);
    if (userBalanceResult.meta && userBalanceResult.meta().logs) {
      console.log("Program logs:");
      userBalanceResult
        .meta()
        .logs()
        .forEach((log: string, index: number) => {
          console.log(`${index + 1}: ${log}`);
        });
    }
    throw new Error("User balance creation failed");
  }

  console.log("User deposited 100 USDC and created balance account!");

  const userBalanceAccount = svm.getAccount(userBalancePda);
  expect(userBalanceAccount).toBeDefined();
  expect(userBalanceAccount!.data.length).toBeGreaterThan(0);

  const accountData = Buffer.from(userBalanceAccount!.data);
  const userBalanceData = UserBalance.decode(accountData);

  // Check that the user balance
  expect(
    userBalanceData.available_quote_balance.eq(new BN(100_000_000))
  ).toBeTrue();
  expect(userBalanceData.owner.equals(user.publicKey)).toBeTrue();
  expect(userBalanceData.market.equals(marketAccountPda)).toBeTrue();

  console.log(
    `   Available USDC: ${
      userBalanceData.available_quote_balance / 1_000_000
    } USDC`
  );
});
