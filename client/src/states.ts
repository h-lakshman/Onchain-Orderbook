import {
  struct,
  u8,
  u16,
  u64,
  publicKey,
  bool,
  rustEnum,
} from "@coral-xyz/borsh";

export const MarketStateSchema = struct([
  publicKey("authority"),
  publicKey("baseMint"),
  publicKey("quoteMint"),
  publicKey("feeAccount"),
  publicKey("baseVault"),
  publicKey("quoteVault"),
  publicKey("marketEvents"),
  publicKey("bids"),
  publicKey("asks"),
  u64("eventHead"),
  u64("eventTail"),
  u64("minOrderSize"),
  u64("tickSize"),
  u64("nextOrderId"),
  u64("lastPrice"),
  u64("volume24h"),
  u16("feeRateBps"),
  u8("bump"),
  bool("isInitialized"),
]);

const OrderSideSchema = rustEnum([struct([], "Buy"), struct([], "Sell")]);

export const InstructionSchema = rustEnum([
  struct([u64("min_order_size"), u64("tick_size")], "InitializeMarket"),
  struct([u64("onramp_quantity")], "CreateUserBalanceAccount"),
  struct(
    [OrderSideSchema.replicate("side"), u64("price"), u64("quantity")],
    "PlaceOrder"
  ),
]);

export const UserBalance = struct([
  publicKey("owner"),
  publicKey("market"),
  u64("available_base_balance"),
  u64("available_quote_balance"),
  u64("locked_base_balance"),
  u64("locked_quote_balance"),
  u64("pending_base_balance"),
  u64("pending_quote_balance"),
]);
