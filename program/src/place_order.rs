use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::state::{
    Event, EventType, MarketEvents, MarketState, Order, OrderBook, Side, UserBalance, MAX_ORDERS,
};

pub fn process_place_order(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    side: Side,
    price: u64,
    quantity: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let bids_info = next_account_info(account_info_iter)?;
    let asks_info = next_account_info(account_info_iter)?;
    let market_events_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    if !user_info.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;
    let mut bids = OrderBook::try_from_slice(&bids_info.data.borrow())?;
    let mut asks = OrderBook::try_from_slice(&asks_info.data.borrow())?;
    let mut market_events = MarketEvents::try_from_slice(&market_events_info.data.borrow())?;
    let clock = Clock::from_account_info(clock_sysvar_info)?;

    let (taker_book, maker_book) = if side == Side::Buy {
        (&mut bids, &mut asks)
    } else {
        (&mut asks, &mut bids)
    };

    let required_base = if side == Side::Sell { quantity } else { 0 };
    let required_quote = if side == Side::Buy { quantity * price } else { 0 };

    if user_balance.available_base_balance < required_base
        || user_balance.available_quote_balance < required_quote
    {
        msg!("Insufficient funds to place order");
        return Err(ProgramError::InsufficientFunds);
    }

    user_balance.available_base_balance -= required_base;
    user_balance.locked_base_balance += required_base;
    user_balance.available_quote_balance -= required_quote;
    user_balance.locked_quote_balance += required_quote;

    let mut remaining_quantity = quantity;

    // Matching logic
    for i in 0..MAX_ORDERS {
        let maker_order = &mut maker_book.orders[i];
        if remaining_quantity == 0 {
            break;
        }
        if !maker_order.is_active {
            continue;
        }

        let price_match = if side == Side::Buy {
            price >= maker_order.price
        } else {
            price <= maker_order.price
        };

        if price_match {
            let fill_quantity =
                std::cmp::min(remaining_quantity, maker_order.quantity - maker_order.filled_quantity);

            if fill_quantity > 0 {
                maker_order.filled_quantity += fill_quantity;
                remaining_quantity -= fill_quantity;

                let maker_fill_event = Event {
                    event_type: EventType::Fill,
                    maker: maker_order.owner,
                    quantity: fill_quantity,
                    order_id: maker_order.order_id,
                    price: maker_order.price,
                    timestamp: clock.unix_timestamp,
                };
                market_events.add_event(maker_fill_event)?;

                if side == Side::Buy {
                    user_balance.locked_quote_balance -= fill_quantity * maker_order.price;
                    user_balance.available_base_balance += fill_quantity;
                } else {
                    user_balance.locked_base_balance -= fill_quantity;
                    user_balance.available_quote_balance += fill_quantity * maker_order.price;
                }

                if maker_order.filled_quantity == maker_order.quantity {
                    maker_order.is_active = false;
                    maker_book.active_orders_count -= 1;
                }
            }
        }
    }

    if remaining_quantity > 0 {
        if taker_book.active_orders_count >= MAX_ORDERS as u64 {
            return Err(ProgramError::Custom(2)); // Order book is full
        }

        // Find the first inactive slot
        let mut new_order_index: Option<usize> = None;
        for i in 0..MAX_ORDERS {
            if !taker_book.orders[i].is_active {
                new_order_index = Some(i);
                break;
            }
        }

        if let Some(index) = new_order_index {
            taker_book.orders[index] = Order {
                order_id: market_state.next_order_id,
                owner: *user_info.key,
                market: *market_info.key,
                side,
                price,
                quantity: remaining_quantity,
                filled_quantity: 0,
                timestamp: clock.unix_timestamp,
                is_active: true,
            };
            taker_book.active_orders_count += 1;
            market_state.next_order_id += 1;
        } else {
            return Err(ProgramError::Custom(2)); // Order book is full (no inactive slots found)
        }
    }

    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;
    bids.serialize(&mut *bids_info.data.borrow_mut())?;
    asks.serialize(&mut *asks_info.data.borrow_mut())?;
    market_events.serialize(&mut *market_events_info.data.borrow_mut())?;
    market_state.serialize(&mut *market_info.data.borrow_mut())?;

    Ok(())
}