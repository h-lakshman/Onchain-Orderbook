import * as borsh from "@coral-xyz/borsh";

export const MarketStateSchema = borsh.struct([
    borsh.publicKey("authority"),
    borsh.publicKey("baseMint"),
    borsh.publicKey("quoteMint"),
    borsh.u64("minOrderSize"),
    borsh.u64("tickSize"),
    borsh.u64("nextOrderId"),
    borsh.bool("isActive"),
  ]);
  
export const InstructionSchema = borsh.rustEnum([
    borsh.struct([
      borsh.u64("min_order_size"),
      borsh.u64("tick_size"),
    ], "InitializeMarket"),
  ]);