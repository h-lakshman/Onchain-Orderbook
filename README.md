# 🚀 On-Chain Orderbook - Solana Native Implementation

A production-ready, native Rust Solana orderbook implementing deferred settlement architecture with comprehensive security features.

## 📋 Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Key Features](#key-features)
- [3-Phase Settlement Flow](#3-phase-settlement-flow)
- [Security Features](#security-features)
- [Instructions](#instructions)
- [Account Structure](#account-structure)
- [Getting Started](#getting-started)
- [Project Structure](#project-structure)

## 🎯 Overview

This orderbook implements a **deferred settlement architecture** similar to production DEXs like Serum/OpenBook, providing:

- **Real token custody** during order placement
- **Virtual settlement** through authority-controlled event processing
- **User-controlled withdrawal** of settled tokens
- **Production-grade security** with comprehensive mint checks

## 🏗️ Architecture

![Architecture Diagram](./architecture.png)

The orderbook follows a **3-phase deferred settlement model**:

### Phase 1: place_order (Token Custody)

- Users transfer real SPL tokens to market vaults
- Orders are matched using price-time priority
- Fill events are logged for later settlement
- Remaining orders are added to the orderbook

### Phase 2: consume_events (Virtual Settlement)

- Authority-controlled batch processing (max 7 events)
- Updates both maker and taker balances per fill event
- Transfers: `locked_balance` → `pending_balance`
- Efficient event array compaction

### Phase 3: settle_balance (Token Withdrawal)

- User-initiated real token withdrawals
- Market authority PDA-signed transfers
- Transfers: Market vaults → User token accounts
- Resets `pending_balance` to zero

## ✨ Key Features

### 🔒 **Security First**

- **Mint Verification**: Prevents wrong token deposits/withdrawals
- **Vault Validation**: Ensures correct market vault usage
- **PDA Verification**: Authority-based access control
- **Account Validation**: Comprehensive ownership checks

### ⚡ **Performance Optimized**

- **Batch Processing**: Handle multiple events efficiently
- **Gas Efficiency**: Authority pays for settlement
- **Memory Management**: Efficient event array compaction
- **Capital Efficiency**: Perfect for high-frequency trading

### 🛠️ **Production Ready**

- **Modular Design**: Clean separation of concerns
- **Error Handling**: Detailed error messages for debugging
- **Flexible Deposits**: Separate base/quote token deposits
- **Account Management**: Automatic PDA account creation

## 🔄 3-Phase Settlement Flow

```
User Places Order → Real Token Transfer → Event Logging
        ↓
Authority Processes Events → Virtual Balance Updates → Event Cleanup
        ↓
User Settles Balance → Real Token Withdrawal → Balance Reset
```

### Benefits of Deferred Settlement:

- **Gas Efficiency**: Authority batch processes events
- **User Control**: Withdraw tokens when convenient
- **Scalability**: Handle multiple trades efficiently
- **Capital Efficiency**: Optimal for high-frequency scenarios

### Mint Verification

```rust
// Verifies user token accounts contain correct mints
user_token_mint == market_state.expected_mint
```

### Vault Validation

```rust
// Ensures operations use correct market vaults
vault_account.key == market_state.vault_key
```

### Authority Control

```rust
// Only authorized crank can process events
signer == market_state.consume_events_authority
```

## 📝 Instructions

### Core Instructions

| Instruction          | Description                                      | Authority Required |
| -------------------- | ------------------------------------------------ | ------------------ |
| `InitializeMarket`   | Create new trading market                        | Market Creator     |
| `DepositQuoteTokens` | Deposit quote tokens (creates account if needed) | User               |
| `DepositBaseTokens`  | Deposit base tokens                              | User               |
| `PlaceOrder`         | Place buy/sell order                             | User               |
| `ConsumeEvents`      | Process settlement events                        | Crank Authority    |
| `SettleBalance`      | Withdraw settled tokens                          | User               |

### Example Usage

```typescript
// 1. Deposit tokens
await program.methods
  .depositQuoteTokens(new BN(1000))
  .accounts({
    user: userKeypair.publicKey,
    userQuoteTokenAccount: userUsdcAccount,
    // ... other accounts
  })
  .rpc();

// 2. Place order
await program.methods
  .placeOrder({ buy: {} }, new BN(100), new BN(10))
  .accounts({
    user: userKeypair.publicKey,
    // ... other accounts
  })
  .rpc();

// 3. Authority processes events
await program.methods
  .consumeEvents()
  .accounts({
    consumeEventsAuthority: crankKeypair.publicKey,
    // ... other accounts
  })
  .rpc();

// 4. User withdraws tokens
await program.methods
  .settleBalance()
  .accounts({
    user: userKeypair.publicKey,
    // ... other accounts
  })
  .rpc();
```

## 🏛️ Account Structure

### MarketState (380 bytes)

```rust
pub struct MarketState {
    pub authority: Pubkey,                    // Market creator
    pub consume_events_authority: Pubkey,     // Crank authority
    pub base_mint: Pubkey,                    // Base token mint
    pub quote_mint: Pubkey,                   // Quote token mint
    pub base_vault: Pubkey,                   // Base token vault
    pub quote_vault: Pubkey,                  // Quote token vault
    // ... additional fields
}
```

### UserBalance (112 bytes)

```rust
pub struct UserBalance {
    pub available_base_balance: u64,   // Available for new orders
    pub available_quote_balance: u64,  // Available for new orders
    pub locked_base_balance: u64,      // Locked in sell orders
    pub locked_quote_balance: u64,     // Locked in buy orders
    pub pending_base_balance: u64,     // Earned tokens (virtual)
    pub pending_quote_balance: u64,    // Earned tokens (virtual)
}
```

### PDA Seeds

```rust
Market: ["market", base_mint, quote_mint]
UserBalance: ["user_balance", user_key, market_key]
Vaults: ["base_vault", market_key] / ["quote_vault", market_key]
```

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+
- Solana CLI 1.16+

### Build & Test

```bash
# Clone repository
git clone <repository-url>
cd Onchain-Orderbook

# Build program
cd program
cargo build-bpf

# Run tests (if available)
cargo test
```

### Deploy

```bash
# Deploy to devnet
solana program deploy target/deploy/program.so --url devnet

# Or deploy to localnet for testing
solana-test-validator
solana program deploy target/deploy/program.so --url localhost
```

## 📁 Project Structure

```
Onchain-Orderbook/
├── program/
│   ├── src/
│   │   ├── lib.rs                    # Program entry point
│   │   ├── state.rs                  # State definitions
│   │   └── instructions/
│   │       ├── mod.rs                # Instruction exports
│   │       ├── initialize_market.rs  # Market creation
│   │       ├── create_user_account.rs # Token deposits
│   │       ├── place_order.rs        # Order placement & matching
│   │       ├── consume_events.rs     # Event processing
│   │       └── settle_balance.rs     # Token withdrawal
│   └── Cargo.toml
└── README.md
```

## 🔮 Future Enhancements

- [ ] Order cancellation functionality
- [ ] Advanced order types (stop-loss, take-profit)
- [ ] Fee collection mechanism
- [ ] Referral program integration
- [ ] Cross-program invocation support
- [ ] Governance token integration

## 🤝 Contributing

Please feel free to submit a Pull Request if you find any mistakes in my implementation.

**Built with ❤️ on Solana**
