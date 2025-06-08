import * as borsh from "@coral-xyz/borsh";

export const MarketStateSchema = borsh.struct([
    borsh.publicKey("authority"),
    borsh.publicKey("baseMint"),
    borsh.publicKey("quoteMint"),
    borsh.publicKey("feeAccount"),
    borsh.publicKey("baseVault"),
    borsh.publicKey("quoteVault"),
    borsh.publicKey("marketEvents"),
    borsh.u64("eventHead"),
    borsh.u64("eventTail"),
    borsh.u64("minOrderSize"),
    borsh.u64("tickSize"),
    borsh.u64("nextOrderId"),
    borsh.u64("totalEvents"),
    borsh.u64("lastPrice"),
    borsh.u64("volume24h"),
    borsh.u16("feeRateBps"),
    borsh.u8("bump"),
    borsh.bool("isInitialized"),
    
  ]);
  
export const InstructionSchema = borsh.rustEnum([
    borsh.struct([
      borsh.u64("min_order_size"),
      borsh.u64("tick_size"),
    ], "InitializeMarket"),
  ]);