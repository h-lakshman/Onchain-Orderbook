use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use solana_program::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MarketState {
    pub authority: Pubkey,
    pub consume_events_authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub fee_account: Pubkey,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub market_events: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub next_order_id: u64,
    pub last_price: u64,
    pub volume_24h: u64,
    pub fee_rate_bps: u16,
    pub bump: u8,
    pub is_initialized: bool,
}

impl MarketState {
    pub const LEN: usize = 10 * 32 + 5 * 8 + 2 + 1 + 1; // 364 bytes
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct UserBalance {
    pub owner: Pubkey,
    pub market: Pubkey,
    pub available_base_balance: u64,
    pub available_quote_balance: u64,
    pub locked_base_balance: u64,
    pub locked_quote_balance: u64,
    pub pending_base_balance: u64,
    pub pending_quote_balance: u64,
}

impl UserBalance {
    pub const LEN: usize = 2 * 32 + 6 * 8; //112 bytes
}

pub const MAX_EVENTS: usize = 512;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct Event {
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub timestamp: i64,
    pub maker_order_id: u64,
    pub quantity: u64,
    pub price: u64,
    pub event_type: EventType,
    pub side: Side,
}

impl Event {
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1; // 98 bytes
}

#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum EventType {
    Fill,
    Out,
}

unsafe impl Pod for EventType {}
unsafe impl Zeroable for EventType {}

pub const MAX_ORDERS: usize = 1024;

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub timestamp: i64,
    pub price: u64,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub side: Side,
}

#[repr(u8)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

unsafe impl Pod for Side {}
unsafe impl Zeroable for Side {}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct OrderBook {
    pub orders: [Event; MAX_ORDERS],
    pub market: Pubkey,
    pub active_orders_count: u64,
    pub side: Side,
}

impl OrderBook {
    pub const LEN: usize = (98 * MAX_ORDERS) + 32 + 8 + 1; // 100,393 bytes (~98KB)
}
#[repr(C)]
#[derive(Debug, Zeroable, Pod, Clone, Copy)]
pub struct MarketEvents {
    pub events: [Event; MAX_EVENTS],
    pub market: Pubkey,
    pub count: u64,
    pub seq_num: u64,
    pub events_to_process: u64,
}

impl MarketEvents {
    pub const LEN: usize = (98 * MAX_EVENTS) + 32 + 8 + 8 + 8; // 50,232 bytes (~49KB)
}
