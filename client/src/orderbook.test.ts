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
import {
  InstructionSchema,
  MarketStateSchema,
  OrderbookSchema,
  UserBalanceSchema,
} from "./states";
import { BN } from "bn.js";

let testEnv: {
  svm: LiteSVM;
  programId: Keypair;
  authority: Keypair;
  consumerEventsAuthority: Keypair;
  user: Keypair;
  taker: Keypair;
  baseAsset: PublicKey;
  quoteAsset: PublicKey;
  userQuoteTokenAccount: PublicKey;
  userBaseTokenAccount: PublicKey;
  takerQuoteTokenAccount: PublicKey;
  takerBaseTokenAccount: PublicKey;
  marketAccountPda: PublicKey;
  bidsPda: PublicKey;
  asksPda: PublicKey;
  marketEventsPda: PublicKey;
  baseVaultPda: PublicKey;
  quoteVaultPda: PublicKey;
  feeAccountPda: PublicKey;
  userBalancePda: PublicKey;
  takerBalancePda: PublicKey;
};

beforeAll(async () => {
  const svm = new LiteSVM();

  const programId = Keypair.generate();
  const programPath =
    process.env.program_path || "../program/target/deploy/orderbook.so";
  console.log("Using program path:", programPath);
  svm.addProgramFromFile(programId.publicKey, programPath);

  const authority = Keypair.generate();
  const consumerEventsAuthority = Keypair.generate();
  svm.airdrop(authority.publicKey, BigInt(100 * LAMPORTS_PER_SOL));
  svm.airdrop(
    consumerEventsAuthority.publicKey,
    BigInt(100 * LAMPORTS_PER_SOL)
  );

  const user = Keypair.generate();
  svm.airdrop(user.publicKey, BigInt(100 * LAMPORTS_PER_SOL));

  const taker = Keypair.generate();
  svm.airdrop(taker.publicKey, BigInt(100 * LAMPORTS_PER_SOL));

  const baseMintKeypair = Keypair.generate();
  const quoteMintKeypair = Keypair.generate();

  const userQuoteTokenAccount = Keypair.generate();
  const userBaseTokenAccount = Keypair.generate();
  const takerQuoteTokenAccount = Keypair.generate();
  const takerBaseTokenAccount = Keypair.generate();

  const userQuoteTokenAccountRent = svm.minimumBalanceForRentExemption(
    BigInt(165)
  );
  const userBaseTokenAccountRent = svm.minimumBalanceForRentExemption(
    BigInt(165)
  );

  // Create base mint
  const createBaseMintAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: baseMintKeypair.publicKey,
    lamports: Number(svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE))),
    space: MINT_SIZE,
    programId: TOKEN_PROGRAM_ID,
  });

  // Create quote mint
  const createQuoteMintAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: quoteMintKeypair.publicKey,
    lamports: Number(svm.minimumBalanceForRentExemption(BigInt(MINT_SIZE))),
    space: MINT_SIZE,
    programId: TOKEN_PROGRAM_ID,
  });

  // Initialize mints
  const initializeBaseMintIx = createInitializeMintInstruction(
    baseMintKeypair.publicKey,
    9,
    authority.publicKey,
    null
  );

  const initializeQuoteMintIx = createInitializeMintInstruction(
    quoteMintKeypair.publicKey,
    6,
    authority.publicKey,
    null
  );

  // Create and initialize user token accounts
  const createUserQuoteTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: userQuoteTokenAccount.publicKey,
    lamports: Number(userQuoteTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const createUserBaseTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: userBaseTokenAccount.publicKey,
    lamports: Number(userBaseTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const initializeUserQuoteTokenAccountIx = createInitializeAccountInstruction(
    userQuoteTokenAccount.publicKey,
    quoteMintKeypair.publicKey,
    user.publicKey
  );

  const initializeUserBaseTokenAccountIx = createInitializeAccountInstruction(
    userBaseTokenAccount.publicKey,
    baseMintKeypair.publicKey,
    user.publicKey
  );

  // Create and initialize taker token accounts
  const createTakerQuoteTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: takerQuoteTokenAccount.publicKey,
    lamports: Number(userQuoteTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const createTakerBaseTokenAccountIx = SystemProgram.createAccount({
    fromPubkey: authority.publicKey,
    newAccountPubkey: takerBaseTokenAccount.publicKey,
    lamports: Number(userBaseTokenAccountRent),
    space: 165,
    programId: TOKEN_PROGRAM_ID,
  });

  const initializeTakerQuoteTokenAccountIx = createInitializeAccountInstruction(
    takerQuoteTokenAccount.publicKey,
    quoteMintKeypair.publicKey,
    taker.publicKey
  );

  const initializeTakerBaseTokenAccountIx = createInitializeAccountInstruction(
    takerBaseTokenAccount.publicKey,
    baseMintKeypair.publicKey,
    taker.publicKey
  );

  // Mint tokens to users
  const mintQuoteToUserIx = createMintToInstruction(
    quoteMintKeypair.publicKey,
    userQuoteTokenAccount.publicKey,
    authority.publicKey,
    1000 * 1_000_000
  );

  const mintBaseToUserIx = createMintToInstruction(
    baseMintKeypair.publicKey,
    userBaseTokenAccount.publicKey,
    authority.publicKey,
    1000 * LAMPORTS_PER_SOL
  );

  const mintQuoteToTakerIx = createMintToInstruction(
    quoteMintKeypair.publicKey,
    takerQuoteTokenAccount.publicKey,
    authority.publicKey,
    1000 * 1_000_000
  );

  const mintBaseToTakerIx = createMintToInstruction(
    baseMintKeypair.publicKey,
    takerBaseTokenAccount.publicKey,
    authority.publicKey,
    1000 * LAMPORTS_PER_SOL
  );

  // Setup mints
  const setupMintsTransaction = new Transaction()
    .add(createBaseMintAccountIx)
    .add(initializeBaseMintIx)
    .add(createQuoteMintAccountIx)
    .add(initializeQuoteMintIx);

  setupMintsTransaction.feePayer = authority.publicKey;
  setupMintsTransaction.recentBlockhash = await svm.latestBlockhash();
  setupMintsTransaction.sign(authority, baseMintKeypair, quoteMintKeypair);

  const setupMintsResult = await svm.sendTransaction(setupMintsTransaction);
  if (
    setupMintsResult &&
    typeof setupMintsResult === "object" &&
    "err" in setupMintsResult
  ) {
    throw new Error("Failed to setup mints");
  }

  // Setup user token accounts
  const setupUserAccountsTransaction = new Transaction()
    .add(createUserQuoteTokenAccountIx)
    .add(initializeUserQuoteTokenAccountIx)
    .add(createUserBaseTokenAccountIx)
    .add(initializeUserBaseTokenAccountIx)
    .add(mintQuoteToUserIx)
    .add(mintBaseToUserIx);

  setupUserAccountsTransaction.feePayer = authority.publicKey;
  setupUserAccountsTransaction.recentBlockhash = await svm.latestBlockhash();
  setupUserAccountsTransaction.sign(
    authority,
    userQuoteTokenAccount,
    userBaseTokenAccount
  );

  const setupUserAccountsResult = await svm.sendTransaction(
    setupUserAccountsTransaction
  );
  if (
    setupUserAccountsResult &&
    typeof setupUserAccountsResult === "object" &&
    "err" in setupUserAccountsResult
  ) {
    throw new Error("Failed to setup user accounts");
  }

  // Setup taker token accounts
  const setupTakerAccountsTransaction = new Transaction()
    .add(createTakerQuoteTokenAccountIx)
    .add(initializeTakerQuoteTokenAccountIx)
    .add(createTakerBaseTokenAccountIx)
    .add(initializeTakerBaseTokenAccountIx)
    .add(mintQuoteToTakerIx)
    .add(mintBaseToTakerIx);

  setupTakerAccountsTransaction.feePayer = authority.publicKey;
  setupTakerAccountsTransaction.recentBlockhash = await svm.latestBlockhash();
  setupTakerAccountsTransaction.sign(
    authority,
    takerQuoteTokenAccount,
    takerBaseTokenAccount
  );

  const setupTakerAccountsResult = await svm.sendTransaction(
    setupTakerAccountsTransaction
  );
  if (
    setupTakerAccountsResult &&
    typeof setupTakerAccountsResult === "object" &&
    "err" in setupTakerAccountsResult
  ) {
    throw new Error("Failed to setup taker accounts");
  }

  console.log(" Test environment setup complete:");
  console.log("   SOL/USDC Market");
  console.log(" User has 1000 USDC and 1000 SOL");
  console.log(" Taker has 1000 USDC and 1000 SOL");

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
    [
      Buffer.from("user_balance"),
      user.publicKey.toBuffer(),
      marketAccountPda.toBuffer(),
    ],
    programId.publicKey
  );

  const [takerBalancePda] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("user_balance"),
      taker.publicKey.toBuffer(),
      marketAccountPda.toBuffer(),
    ],
    programId.publicKey
  );

  testEnv = {
    svm,
    programId,
    authority,
    consumerEventsAuthority,
    user,
    taker,
    baseAsset,
    quoteAsset,
    userQuoteTokenAccount: userQuoteTokenAccount.publicKey,
    userBaseTokenAccount: userBaseTokenAccount.publicKey,
    takerQuoteTokenAccount: takerQuoteTokenAccount.publicKey,
    takerBaseTokenAccount: takerBaseTokenAccount.publicKey,
    marketAccountPda,
    bidsPda,
    asksPda,
    marketEventsPda,
    baseVaultPda,
    quoteVaultPda,
    feeAccountPda,
    userBalancePda,
    takerBalancePda,
  };
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
    consumerEventsAuthority,
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
      {
        pubkey: consumerEventsAuthority.publicKey,
        isSigner: false,
        isWritable: false,
      },
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
  expect(marketState.base_mint.equals(baseAsset)).toBeTrue();
  expect(marketState.quote_mint.equals(quoteAsset)).toBeTrue();
  expect(marketState.min_order_size.eq(minOrderSize)).toBeTrue();
  expect(marketState.tick_size.eq(tickSize)).toBeTrue();
  expect(marketState.is_initialized).toBeTrue();

  console.log(" Market initialized successfully!");
});

test("Create User Quote Balance Account and Deposit Funds", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    quoteVaultPda,
    userBalancePda,
    userQuoteTokenAccount,
  } = testEnv;

  // Deposit 100 USDC (6 decimals)
  const depositAmount = new BN(100 * 1_000_000);

  const userBalanceDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      DepositQuoteTokens: {
        quantity: depositAmount,
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

  const userBalanceAccount = svm.getAccount(userBalancePda);
  expect(userBalanceAccount).toBeDefined();
  expect(userBalanceAccount!.data.length).toBeGreaterThan(0);

  const accountData = Buffer.from(userBalanceAccount!.data);
  const userBalanceData = UserBalanceSchema.decode(accountData);

  console.log("User deposited 100 USDC and created balance account!");
  console.log(
    `   Available USDC: ${
      userBalanceData.available_quote_balance / 1_000_000
    } USDC`
  );

  expect(
    userBalanceData.available_quote_balance.eq(new BN(100 * 1_000_000))
  ).toBeTrue();
  expect(userBalanceData.owner.equals(user.publicKey)).toBeTrue();
  expect(userBalanceData.market.equals(marketAccountPda)).toBeTrue();
});

test("Create User Base Balance Account and Deposit Funds", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    baseVaultPda,
    userBalancePda,
    userBaseTokenAccount,
  } = testEnv;

  // Deposit 100 SOL (9 decimals)
  const depositAmount = new BN(100 * LAMPORTS_PER_SOL);

  const userBalanceDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      DepositBaseTokens: {
        quantity: depositAmount,
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
      { pubkey: userBaseTokenAccount, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
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

  console.log("User deposited 100 SOL to existing balance account!");

  const userBalanceAccount = svm.getAccount(userBalancePda);
  expect(userBalanceAccount).toBeDefined();
  expect(userBalanceAccount!.data.length).toBeGreaterThan(0);

  const accountData = Buffer.from(userBalanceAccount!.data);
  const userBalanceData = UserBalanceSchema.decode(accountData);

  expect(
    userBalanceData.available_base_balance.eq(new BN(100 * LAMPORTS_PER_SOL))
  ).toBeTrue();
  expect(userBalanceData.owner.equals(user.publicKey)).toBeTrue();
  expect(userBalanceData.market.equals(marketAccountPda)).toBeTrue();

  console.log(
    `   Available SOL: ${
      userBalanceData.available_base_balance / LAMPORTS_PER_SOL
    } SOL`
  );
});

test("Place buy order,Place sell order, Check market events", async () => {
  const {
    svm,
    marketAccountPda,
    userBalancePda,
    user,
    bidsPda,
    asksPda,
    marketEventsPda,
    userBaseTokenAccount,
    userQuoteTokenAccount,
    baseVaultPda,
    quoteVaultPda,
    programId,
    consumerEventsAuthority,
    taker,
    takerBalancePda,
  } = testEnv;

  // Check user's balance before placing buy order
  const userBalanceBeforeBuy = svm.getAccount(userBalancePda);
  if (!userBalanceBeforeBuy) {
    throw new Error("User balance account not found");
  }
  const userBalanceDataBeforeBuy = UserBalanceSchema.decode(
    Buffer.from(userBalanceBeforeBuy.data)
  );
  console.log("\nUser Balance Before Buy Order:");
  console.log(
    `Available Base: ${userBalanceDataBeforeBuy.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Available Quote: ${userBalanceDataBeforeBuy.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  // Place buy order with 50 USDC price for 1 SOL
  const price = new BN(50_000_000); // 50 USDC
  const quantity = new BN(LAMPORTS_PER_SOL); // 1 SOL in base units

  const placeOrderDataBuffer = Buffer.alloc(18);
  InstructionSchema.encode(
    {
      PlaceOrder: {
        side: 0,
        price: price,
        quantity: quantity,
      },
    },
    placeOrderDataBuffer
  );

  const userBalanceAccount = svm.getAccount(userBalancePda);
  if (!userBalanceAccount) {
    throw new Error("User balance account not found");
  }

  const accountData = Buffer.from(userBalanceAccount.data);
  const userBalanceData = UserBalanceSchema.decode(accountData);

  const placeOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: placeOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      { pubkey: userBaseTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userQuoteTokenAccount, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const placeOrderTx = new Transaction().add(placeOrderIx);
  placeOrderTx.feePayer = user.publicKey;
  placeOrderTx.recentBlockhash = await svm.latestBlockhash();
  placeOrderTx.sign(user);

  const orderResult = await svm.sendTransaction(placeOrderTx);

  if (orderResult && typeof orderResult === "object" && "err" in orderResult) {
    console.error("Place order failed:", orderResult);
    if (orderResult.meta && orderResult.meta().logs) {
      console.log("Program logs:");
      orderResult
        .meta()
        .logs()
        .forEach((log: string, index: number) => {
          console.log(`${index + 1}: ${log}`);
        });
    }
    throw new Error("Place order failed");
  }

  console.log("Buy order placed successfully!");

  const bidsAccount = svm.getAccount(bidsPda);
  expect(bidsAccount).toBeDefined();
  expect(bidsAccount!.data.length).toBeGreaterThan(0);

  const bidsAccountData = Buffer.from(bidsAccount!.data);
  const minSize = 32 + 1 + 4 + 8; // 45 bytes for empty OrderBook

  if (bidsAccountData.length >= minSize) {
    const ordersLen = bidsAccountData.readUInt32LE(33);
    console.log(`Orders in bids: ${ordersLen}`);

    const actualSize = minSize + ordersLen * 105;
    const actualData = bidsAccountData.subarray(0, actualSize);
    const bidsData = OrderbookSchema.decode(actualData);

    expect(bidsData.orders.length).toBe(1);
    const buyOrder = bidsData.orders[0];

    expect(buyOrder.order_id.eq(new BN(1))).toBeTrue(); // First order
    expect(buyOrder.owner.equals(user.publicKey)).toBeTrue();
    expect(buyOrder.market.equals(marketAccountPda)).toBeTrue();
    expect(buyOrder.side).toBe(0);
    expect(buyOrder.price.eq(price)).toBeTrue();
    expect(buyOrder.quantity.eq(quantity)).toBeTrue();
    expect(buyOrder.filled_quantity.eq(new BN(0))).toBeTrue();
    console.log("Buy order verified in bids orderbook");
  }

  // Create taker balance account and deposit base tokens for sell order
  const depositBaseDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      DepositBaseTokens: {
        quantity: new BN(100 * LAMPORTS_PER_SOL), // 100 SOL
      },
    },
    depositBaseDataBuffer
  );

  const createTakerBalanceIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: depositBaseDataBuffer,
    keys: [
      { pubkey: taker.publicKey, isSigner: true, isWritable: true },
      { pubkey: takerBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: false },
      {
        pubkey: testEnv.takerBaseTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const takerBalanceTx = new Transaction().add(createTakerBalanceIx);
  takerBalanceTx.feePayer = taker.publicKey;
  takerBalanceTx.recentBlockhash = await svm.latestBlockhash();
  takerBalanceTx.sign(taker);

  const takerBalanceResult = await svm.sendTransaction(takerBalanceTx);
  if (
    takerBalanceResult &&
    typeof takerBalanceResult === "object" &&
    "err" in takerBalanceResult
  ) {
    throw new Error("Taker balance creation failed");
  }

  console.log("Taker balance account created");

  // Check taker's balance before placing sell order
  const takerBalanceBeforeSell = svm.getAccount(takerBalancePda);
  if (!takerBalanceBeforeSell) {
    throw new Error("Taker balance account not found");
  }
  const takerBalanceDataBeforeSell = UserBalanceSchema.decode(
    Buffer.from(takerBalanceBeforeSell.data)
  );
  console.log("\nTaker Balance Before Sell Order:");
  console.log(
    `Available Base: ${takerBalanceDataBeforeSell.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Available Quote: ${takerBalanceDataBeforeSell.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  // Place sell order with 45 USDC price for 1 SOL
  const sellPrice = new BN(45_000_000); // 45 USDC
  const sellQuantity = new BN(LAMPORTS_PER_SOL); // 1 SOL in base units

  // Place sell order using taker account
  const placeSellOrderBuffer = Buffer.alloc(18);
  InstructionSchema.encode(
    {
      PlaceOrder: {
        side: 1,
        price: sellPrice,
        quantity: sellQuantity,
      },
    },
    placeSellOrderBuffer
  );

  const placeSellOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: placeSellOrderBuffer,
    keys: [
      { pubkey: taker.publicKey, isSigner: true, isWritable: false },
      { pubkey: takerBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      {
        pubkey: testEnv.takerBaseTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: testEnv.takerQuoteTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const sellOrderTx = new Transaction().add(placeSellOrderIx);
  sellOrderTx.feePayer = taker.publicKey;
  sellOrderTx.recentBlockhash = await svm.latestBlockhash();
  sellOrderTx.sign(taker);

  const sellResult = await svm.sendTransaction(sellOrderTx);
  if (sellResult && typeof sellResult === "object" && "err" in sellResult) {
    throw new Error("Sell order failed");
  }

  console.log("Sell order placed successfully!");

  const asksAccount = svm.getAccount(asksPda);
  expect(asksAccount).toBeDefined();
  expect(asksAccount!.data.length).toBeGreaterThan(0);

  const asksAccountData = Buffer.from(asksAccount!.data);

  if (asksAccountData.length >= minSize) {
    const asksOrdersLen = asksAccountData.readUInt32LE(33);
    console.log(`Orders in asks: ${asksOrdersLen}`);

    const asksActualSize = minSize + asksOrdersLen * 105;
    const asksActualData = asksAccountData.subarray(0, asksActualSize);
    const asksData = OrderbookSchema.decode(asksActualData);

    //  sell price 45 < buy price (50), the order matches immediately
    expect(asksData.orders.length).toBe(0);
    console.log("Sell order matched immediately with existing buy order");
  }

  // balance check before consume_events
  const userBalanceBeforeConsume = svm.getAccount(userBalancePda);
  const userBalanceDataBefore = UserBalanceSchema.decode(
    Buffer.from(userBalanceBeforeConsume!.data)
  );

  console.log("\nser Balance Before Consume Events:");
  console.log(
    `Available Base: ${userBalanceDataBefore.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Available Quote: ${userBalanceDataBefore.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Pending Base: ${userBalanceDataBefore.pending_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Pending Quote: ${userBalanceDataBefore.pending_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Locked Base: ${userBalanceDataBefore.locked_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Locked Quote: ${userBalanceDataBefore.locked_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  const consumeEventsDataBuffer = Buffer.alloc(1);
  InstructionSchema.encode(
    {
      ConsumeEvents: {},
    },
    consumeEventsDataBuffer
  );

  const consumeEventsIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: consumeEventsDataBuffer,
    keys: [
      {
        pubkey: consumerEventsAuthority.publicKey,
        isSigner: true,
        isWritable: false,
      },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true }, // Maker
      { pubkey: takerBalancePda, isSigner: false, isWritable: true }, // Taker
      {
        pubkey: testEnv.userBaseTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: testEnv.userQuoteTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: testEnv.takerBaseTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: testEnv.takerQuoteTokenAccount,
        isSigner: false,
        isWritable: true,
      },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
    ],
  });

  const consumeEventsTx = new Transaction().add(consumeEventsIx);
  consumeEventsTx.feePayer = consumerEventsAuthority.publicKey;
  consumeEventsTx.recentBlockhash = await svm.latestBlockhash();
  consumeEventsTx.sign(consumerEventsAuthority);

  const consumeResult = await svm.sendTransaction(consumeEventsTx);
  if (
    consumeResult &&
    typeof consumeResult === "object" &&
    "err" in consumeResult
  ) {
    console.error("Consume events failed:", consumeResult);
    if (consumeResult.meta && consumeResult.meta().logs) {
      console.log("Program logs:");
      consumeResult
        .meta()
        .logs()
        .forEach((log: string, index: number) => {
          console.log(`${index + 1}: ${log}`);
        });
    }
    throw new Error("Consume events failed");
  }

  console.log("Events consumed successfully!");

  // check pending balance in user_balance pda after consume events
  const userBalanceAfterConsume = svm.getAccount(userBalancePda);
  const userBalanceDataAfter = UserBalanceSchema.decode(
    Buffer.from(userBalanceAfterConsume!.data)
  );

  console.log("\n📊 User Balance After Consume Events:");
  console.log(
    `Available Base: ${userBalanceDataAfter.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Available Quote: ${userBalanceDataAfter.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Pending Base: ${userBalanceDataAfter.pending_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Pending Quote: ${userBalanceDataAfter.pending_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Locked Base: ${userBalanceDataAfter.locked_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Locked Quote: ${userBalanceDataAfter.locked_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  expect(
    userBalanceDataAfter.pending_base_balance.eq(new BN(LAMPORTS_PER_SOL))
  ).toBeTrue();
  expect(userBalanceDataAfter.pending_quote_balance.eq(new BN(0))).toBeTrue();
  expect(userBalanceDataAfter.locked_base_balance.eq(new BN(0))).toBeTrue();
  expect(userBalanceDataAfter.locked_quote_balance.eq(new BN(0))).toBeTrue();

  console.log("Pending and locked balances verified correctly!");
  console.log("\nComplete orderbook flow tested successfully!");
});

test("Settle Balance for User", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    userBalancePda,
    userBaseTokenAccount,
    userQuoteTokenAccount,
    baseVaultPda,
    quoteVaultPda,
    baseAsset,
    quoteAsset,
  } = testEnv;

  const [marketAuthorityPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), baseAsset.toBuffer(), quoteAsset.toBuffer()],
    programId.publicKey
  );

  const userBalanceBefore = svm.getAccount(userBalancePda);
  const userBalanceDataBefore = UserBalanceSchema.decode(
    Buffer.from(userBalanceBefore!.data)
  );
  console.log("\nUser Balance Before Settle:");
  console.log(
    `Pending Base: ${userBalanceDataBefore.pending_base_balance.toString()}`
  );
  console.log(
    `Pending Quote: ${userBalanceDataBefore.pending_quote_balance.toString()}`
  );

  const settleBalanceDataBuffer = Buffer.alloc(1);
  InstructionSchema.encode({ SettleBalance: {} }, settleBalanceDataBuffer);

  const settleBalanceIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: settleBalanceDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: true },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: false },
      { pubkey: marketAuthorityPda, isSigner: false, isWritable: false },
      { pubkey: userBaseTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userQuoteTokenAccount, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
  });

  const settleTx = new Transaction().add(settleBalanceIx);
  settleTx.feePayer = user.publicKey;
  settleTx.recentBlockhash = await svm.latestBlockhash();
  settleTx.sign(user);

  const settleResult = await svm.sendTransaction(settleTx);
  if (
    settleResult &&
    typeof settleResult === "object" &&
    "err" in settleResult
  ) {
    console.error("Settle balance failed:", settleResult);
    throw new Error("Settle balance failed");
  }

  const userBalanceAfter = svm.getAccount(userBalancePda);
  const userBalanceDataAfter = UserBalanceSchema.decode(
    Buffer.from(userBalanceAfter!.data)
  );
  console.log("\nUser Balance After Settle:");
  console.log(
    `Pending Base: ${userBalanceDataAfter.pending_base_balance.toString()}`
  );
  console.log(
    `Pending Quote: ${userBalanceDataAfter.pending_quote_balance.toString()}`
  );

  expect(userBalanceDataAfter.pending_base_balance.eq(new BN(0))).toBeTrue();
  expect(userBalanceDataAfter.pending_quote_balance.eq(new BN(0))).toBeTrue();
  console.log("Settle balance test passed!");
});

test("Cancel buy order", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    userBalancePda,
    bidsPda,
    asksPda,
    marketEventsPda,
    userBaseTokenAccount,
    userQuoteTokenAccount,
    baseVaultPda,
    quoteVaultPda,
  } = testEnv;

  // place order to cancel it
  const price = new BN(30_000_000);
  const quantity = new BN(LAMPORTS_PER_SOL); // 1 SOL

  const placeOrderDataBuffer = Buffer.alloc(18);
  InstructionSchema.encode(
    {
      PlaceOrder: {
        side: 0, // Buy
        price: price,
        quantity: quantity,
      },
    },
    placeOrderDataBuffer
  );

  const placeOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: placeOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      { pubkey: userBaseTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userQuoteTokenAccount, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const placeOrderTx = new Transaction().add(placeOrderIx);
  placeOrderTx.feePayer = user.publicKey;
  placeOrderTx.recentBlockhash = await svm.latestBlockhash();
  placeOrderTx.sign(user);

  await svm.sendTransaction(placeOrderTx);
  console.log("Buy order placed for cancellation test");

  const userBalanceBefore = svm.getAccount(userBalancePda);
  const userBalanceDataBefore = UserBalanceSchema.decode(
    Buffer.from(userBalanceBefore!.data)
  );

  console.log("\nUser Balance Before Cancel:");
  console.log(
    `Available Quote: ${userBalanceDataBefore.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Locked Quote: ${userBalanceDataBefore.locked_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  const marketAccount = svm.getAccount(marketAccountPda);
  const marketData = MarketStateSchema.decode(Buffer.from(marketAccount!.data));
  const orderIdToCancel = marketData.next_order_id.sub(new BN(1)); // Last order ID

  console.log(`Trying to cancel order ID: ${orderIdToCancel.toString()}`);

  const cancelOrderDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      CancelOrder: {
        order_id: orderIdToCancel,
      },
    },
    cancelOrderDataBuffer
  );

  const cancelOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: cancelOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const cancelOrderTx = new Transaction().add(cancelOrderIx);
  cancelOrderTx.feePayer = user.publicKey;
  cancelOrderTx.recentBlockhash = await svm.latestBlockhash();
  cancelOrderTx.sign(user);

  const cancelResult = await svm.sendTransaction(cancelOrderTx);
  if (
    cancelResult &&
    typeof cancelResult === "object" &&
    "err" in cancelResult
  ) {
    console.error("Cancel order failed:", cancelResult);
    if (cancelResult.meta && cancelResult.meta().logs) {
      console.log("Program logs:");
      cancelResult
        .meta()
        .logs()
        .forEach((log: string, index: number) => {
          console.log(`${index + 1}: ${log}`);
        });
    }
    throw new Error("Cancel order failed");
  }

  console.log("Buy order cancelled successfully!");

  const userBalanceAfter = svm.getAccount(userBalancePda);
  const userBalanceDataAfter = UserBalanceSchema.decode(
    Buffer.from(userBalanceAfter!.data)
  );

  console.log("\nUser Balance After Cancel:");
  console.log(
    `Available Quote: ${userBalanceDataAfter.available_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );
  console.log(
    `Locked Quote: ${userBalanceDataAfter.locked_quote_balance.div(
      new BN(1_000_000)
    )} USDC`
  );

  const expectedAvailableQuote =
    userBalanceDataBefore.available_quote_balance.add(
      userBalanceDataBefore.locked_quote_balance
    );
  expect(
    userBalanceDataAfter.available_quote_balance.eq(expectedAvailableQuote)
  ).toBeTrue();
  expect(userBalanceDataAfter.locked_quote_balance.eq(new BN(0))).toBeTrue();

  const bidsAccount = svm.getAccount(bidsPda);
  const bidsAccountData = Buffer.from(bidsAccount!.data);
  const minSize = 32 + 1 + 4 + 8;

  if (bidsAccountData.length >= minSize) {
    const ordersLen = bidsAccountData.readUInt32LE(33);
    const actualSize = minSize + ordersLen * 105;
    const actualData = bidsAccountData.subarray(0, actualSize);
    const bidsData = OrderbookSchema.decode(actualData);

    const cancelledOrderExists = bidsData.orders.some(
      (order: any) =>
        order.order_id.eq(orderIdToCancel) && order.owner.equals(user.publicKey)
    );
    expect(cancelledOrderExists).toBeFalse();
  }

  console.log("Buy order cancellation test passed!");
});

test("Cancel sell order", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    userBalancePda,
    bidsPda,
    asksPda,
    marketEventsPda,
    userBaseTokenAccount,
    userQuoteTokenAccount,
    baseVaultPda,
    quoteVaultPda,
  } = testEnv;

  // place a sell order to cancel
  const price = new BN(45_000_000); // 45 USDC
  const quantity = new BN(LAMPORTS_PER_SOL); // 1 SOL

  const placeOrderDataBuffer = Buffer.alloc(18);
  InstructionSchema.encode(
    {
      PlaceOrder: {
        side: 1, // Sell
        price: price,
        quantity: quantity,
      },
    },
    placeOrderDataBuffer
  );

  const placeOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: placeOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      { pubkey: userBaseTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userQuoteTokenAccount, isSigner: false, isWritable: true },
      { pubkey: baseVaultPda, isSigner: false, isWritable: true },
      { pubkey: quoteVaultPda, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const placeOrderTx = new Transaction().add(placeOrderIx);
  placeOrderTx.feePayer = user.publicKey;
  placeOrderTx.recentBlockhash = await svm.latestBlockhash();
  placeOrderTx.sign(user);

  await svm.sendTransaction(placeOrderTx);
  console.log("Sell order placed for cancellation test");

  const userBalanceBefore = svm.getAccount(userBalancePda);
  const userBalanceDataBefore = UserBalanceSchema.decode(
    Buffer.from(userBalanceBefore!.data)
  );

  console.log("\nUser Balance Before Cancel:");
  console.log(
    `Available Base: ${userBalanceDataBefore.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Locked Base: ${userBalanceDataBefore.locked_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );

  const marketAccount = svm.getAccount(marketAccountPda);
  const marketData = MarketStateSchema.decode(Buffer.from(marketAccount!.data));
  const orderIdToCancel = marketData.next_order_id.sub(new BN(1)); // Last order ID

  const cancelOrderDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      CancelOrder: {
        order_id: orderIdToCancel,
      },
    },
    cancelOrderDataBuffer
  );

  const cancelOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: cancelOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const cancelOrderTx = new Transaction().add(cancelOrderIx);
  cancelOrderTx.feePayer = user.publicKey;
  cancelOrderTx.recentBlockhash = await svm.latestBlockhash();
  cancelOrderTx.sign(user);

  const cancelResult = await svm.sendTransaction(cancelOrderTx);
  if (
    cancelResult &&
    typeof cancelResult === "object" &&
    "err" in cancelResult
  ) {
    console.error("Cancel sell order failed:", cancelResult);
    throw new Error("Cancel sell order failed");
  }

  console.log("Sell order cancelled successfully!");

  const userBalanceAfter = svm.getAccount(userBalancePda);
  const userBalanceDataAfter = UserBalanceSchema.decode(
    Buffer.from(userBalanceAfter!.data)
  );

  console.log("\nUser Balance After Cancel:");
  console.log(
    `Available Base: ${userBalanceDataAfter.available_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );
  console.log(
    `Locked Base: ${userBalanceDataAfter.locked_base_balance.div(
      new BN(LAMPORTS_PER_SOL)
    )} SOL`
  );

  const expectedAvailableBase =
    userBalanceDataBefore.available_base_balance.add(
      userBalanceDataBefore.locked_base_balance
    );
  expect(
    userBalanceDataAfter.available_base_balance.eq(expectedAvailableBase)
  ).toBeTrue();
  expect(userBalanceDataAfter.locked_base_balance.eq(new BN(0))).toBeTrue();

  const asksAccount = svm.getAccount(asksPda);
  const asksAccountData = Buffer.from(asksAccount!.data);
  const minSize = 32 + 1 + 4 + 8;

  if (asksAccountData.length >= minSize) {
    const ordersLen = asksAccountData.readUInt32LE(33);
    const actualSize = minSize + ordersLen * 105;
    const actualData = asksAccountData.subarray(0, actualSize);
    const asksData = OrderbookSchema.decode(actualData);

    const cancelledOrderExists = asksData.orders.some(
      (order: any) =>
        order.order_id.eq(orderIdToCancel) && order.owner.equals(user.publicKey)
    );
    expect(cancelledOrderExists).toBeFalse();
  }

  console.log("Sell order cancellation test passed!");
});

test("Cancel order - order not found", async () => {
  const {
    svm,
    programId,
    user,
    marketAccountPda,
    userBalancePda,
    bidsPda,
    asksPda,
    marketEventsPda,
  } = testEnv;

  const nonExistentOrderId = new BN(999999);

  const cancelOrderDataBuffer = Buffer.alloc(9);
  InstructionSchema.encode(
    {
      CancelOrder: {
        order_id: nonExistentOrderId,
      },
    },
    cancelOrderDataBuffer
  );

  const cancelOrderIx = new TransactionInstruction({
    programId: programId.publicKey,
    data: cancelOrderDataBuffer,
    keys: [
      { pubkey: user.publicKey, isSigner: true, isWritable: false },
      { pubkey: userBalancePda, isSigner: false, isWritable: true },
      { pubkey: marketAccountPda, isSigner: false, isWritable: true },
      { pubkey: bidsPda, isSigner: false, isWritable: true },
      { pubkey: asksPda, isSigner: false, isWritable: true },
      { pubkey: marketEventsPda, isSigner: false, isWritable: true },
      {
        pubkey: new PublicKey("SysvarC1ock11111111111111111111111111111111"),
        isSigner: false,
        isWritable: false,
      },
    ],
  });

  const cancelOrderTx = new Transaction().add(cancelOrderIx);
  cancelOrderTx.feePayer = user.publicKey;
  cancelOrderTx.recentBlockhash = await svm.latestBlockhash();
  cancelOrderTx.sign(user);

  const cancelResult = await svm.sendTransaction(cancelOrderTx);

  expect(cancelResult).toBeDefined();
  expect(typeof cancelResult === "object" && "err" in cancelResult).toBeTrue();

  console.log("Cancel non-existent order correctly failed!");
});
