use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MarketState {
    pub authority: Pubkey,        
    pub base_mint: Pubkey,        
    pub quote_mint: Pubkey,       
    pub fee_account: Pubkey,      
    pub base_vault: Pubkey,       
    pub quote_vault: Pubkey,      
    pub market_events: Pubkey,
    pub bids:Pubkey,
    pub asks:Pubkey,   
    pub event_head: u64,          
    pub event_tail: u64,          
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
    pub const LEN: usize = 9 * 32 + 7 * 8 + 2 + 1 + 1; // 348 bytes
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
    pub const LEN: usize = 2 * 32 + 6 * 8; // 112 bytes
}

pub const MAX_EVENTS: usize = 512;

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub struct Event {
    pub event_type: EventType,
    pub maker: Pubkey,
    pub quantity: u64,
    pub order_id: u64,
    pub price: u64,
    pub timestamp: i64,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            event_type: EventType::Fill,
            maker: Pubkey::default(),
            quantity: 0,
            order_id: 0,
            price: 0,
            timestamp: 0,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq)]
pub enum EventType {
    Fill,  
    Out,   
}

pub const MAX_ORDERS: usize = 1024; // Max possible orders

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub struct Order {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub timestamp: i64,
    pub is_active: bool,
}

impl Default for Order {
    fn default() -> Self {
        Self {
            order_id: 0,
            owner: Pubkey::default(),
            market: Pubkey::default(),
            side: Side::Buy,
            price: 0,
            quantity: 0,
            filled_quantity: 0,
            timestamp: 0,
            is_active: false,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct OrderBook {
    pub market: Pubkey,
    pub side: Side,
    pub orders: [Order; MAX_ORDERS],
    pub active_orders_count: u64,
}

impl OrderBook {
    pub const LEN: usize = 32 + 1 + (106 * MAX_ORDERS) + 8; // ~10.2KB

    pub fn new(market: Pubkey, side: Side) -> Self {
        Self {
            market,
            side,
            orders: [Order::default(); MAX_ORDERS],
            active_orders_count: 0,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MarketEvents {
    pub market: Pubkey,
    pub head: u64,
    pub count: u64,
    pub seq_num: u64,
    pub events: [Event; MAX_EVENTS],
}

impl MarketEvents {
    pub const LEN: usize = 32 + 8 + 8 + 8 + (65 * MAX_EVENTS); // ~33KB

    pub fn new(market: Pubkey) -> Self {
        Self {
            market,
            head: 0,
            count: 0,
            seq_num: 0,
            events: [Event::default(); MAX_EVENTS],
        }
    }

    pub fn add_event(&mut self, event: Event) -> ProgramResult {
        if self.count >= MAX_EVENTS as u64 {
            return Err(ProgramError::Custom(1)); // Event queue is full
        }
        let tail = (self.head + self.count) % (MAX_EVENTS as u64);
        self.events[tail as usize] = event;
        self.count += 1;
        self.seq_num += 1;
        Ok(())
    }
}