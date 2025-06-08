use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MarketState {
    pub authority: Pubkey,        
    pub base_mint: Pubkey,        
    pub quote_mint: Pubkey,       
    pub fee_account: Pubkey,      
    pub base_vault: Pubkey,       
    pub quote_vault: Pubkey,      
    pub market_events: Pubkey,    
    pub event_head: u64,          
    pub event_tail: u64,          
    pub min_order_size: u64,      
    pub tick_size: u64,           
    pub next_order_id: u64,       
    pub total_events: u64,        
    pub last_price: u64,          
    pub volume_24h: u64,          
    pub fee_rate_bps: u16,        
    pub bump: u8,                 
    pub is_initialized: bool,     
}

impl MarketState {
    pub const LEN: usize = 7 * 32 + 6 * 8 + 2 + 2 + 16; // 292 bytes,extra 16 bytes for borsh overhead
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
    pub const LEN: usize = 32 + 32 + 8 + 8 + 8 + 8 + 8 + 8; // 112 bytes
}
pub const MAX_EVENTS: usize = 1000; //scale it accordingly

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct MarketEvents {
    pub market: Pubkey,
    pub events: Vec<Event>,  
    pub head: u64,
    pub tail: u64,
}

impl MarketEvents {
    pub fn calculate_size(num_events: usize) -> usize {
        32 +           
        4 +           
        (Event::LEN * num_events) + 
        8 +            
        8              
    }
    
    pub const MAX_LEN: usize = 32 + 4 + (Event::LEN * MAX_EVENTS) + 8 + 8; // ~97KB
    
    pub const MIN_LEN: usize = 32 + 4 + 8 + 8; // 52 bytes
    
    pub fn new(market: Pubkey) -> Self {
        Self {
            market,
            events: Vec::new(),
            head: 0,
            tail: 0,
        }
    }
    
    pub fn add_event(&mut self, event: Event) -> Result<(), &'static str> {
        if self.events.len() >= MAX_EVENTS {
            return Err("Event queue is full");
        }
        self.events.push(event);
        Ok(())
    }
    
    pub fn is_full(&self) -> bool {
        self.events.len() >= MAX_EVENTS
    }
    
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub struct Event {
    pub event_type: EventType,
    pub order_id: u64,
    pub maker: Pubkey,
    pub taker: Pubkey,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

impl Event {
    pub const LEN: usize = 1 + 8 + 32 + 32 + 8 + 8 + 8; // 97 bytes
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, Copy)]
pub enum EventType {
    Fill,  
    Out,   
}

#[derive(BorshDeserialize, BorshSerialize, Debug)]
pub struct Order {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: OrderSide,
    pub price: u64,
    pub quantity: u64,
    pub filled_quantity: u64,
    pub timestamp: i64,
    pub is_active: bool,
}

impl Order {
    pub const LEN: usize = 8 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 1; // 106 bytes
}

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}