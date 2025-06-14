import {
  struct,
  u8,
  u16,
  u64,
  publicKey,
  bool,
  rustEnum,
  vec,
  i64,
  array,
} from "@coral-xyz/borsh";

export const MarketStateSchema = struct([
  publicKey("authority"),
  publicKey("consume_events_authority"),
  publicKey("base_mint"),
  publicKey("quote_mint"),
  publicKey("fee_account"),
  publicKey("base_vault"),
  publicKey("quote_vault"),
  publicKey("market_events"),
  publicKey("bids"),
  publicKey("asks"),
  u64("min_order_size"),
  u64("tick_size"),
  u64("next_order_id"),
  u64("last_price"),
  u64("volume_24h"),
  u16("fee_rate_bps"),
  u8("bump"),
  bool("is_initialized"),
]);

export const OrderSideSchema = rustEnum([
  struct([], "Buy"),
  struct([], "Sell"),
]);

export const InstructionSchema = rustEnum([
  struct([u64("min_order_size"), u64("tick_size")], "InitializeMarket"),
  struct([u64("quantity")], "DepositQuoteTokens"),
  struct([u64("quantity")], "DepositBaseTokens"),
  struct([u8("side"), u64("price"), u64("quantity")], "PlaceOrder"),
  struct([], "ConsumeEvents"),
  struct([], "SettleBalance"),
  struct([u64("order_id")], "CancelOrder"),
]);

export const UserBalanceSchema = struct([
  publicKey("owner"),
  publicKey("market"),
  u64("available_base_balance"),
  u64("available_quote_balance"),
  u64("locked_base_balance"),
  u64("locked_quote_balance"),
  u64("pending_base_balance"),
  u64("pending_quote_balance"),
]);

export const OrderSchema = struct([
  publicKey("owner"),
  publicKey("market"),
  i64("timestamp"),
  u64("order_id"),
  u64("price"),
  u64("quantity"),
  u64("filled_quantity"),
  u8("side"),
]);

export const OrderbookSchema = struct([
  array(OrderSchema, 1024, "orders"),
  publicKey("market"),
  u64("active_orders_count"),
  u8("side"),
]);

export const MARKET_EVENT_LEN = 50232; // bytes
export const ORDERBOOK_LEN = 107561; // bytes
