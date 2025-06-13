use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::state::{Event, EventType, MarketEvents, MarketState, OrderBook, UserBalance};

pub fn process_cancel_order(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    order_id: u64,
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

    let user_account_seeds = &[
        b"user_balance",
        user_info.key.as_ref(),
        market_info.key.as_ref(),
    ];
    let (user_balance_pda, _user_balance_bump) =
        Pubkey::find_program_address(user_account_seeds, program_id);

    if user_balance_pda != *user_balance_info.key {
        msg!("Invalid user account. Expected PDA: {}", user_balance_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;

    if user_balance.owner != *user_info.key {
        msg!("User balance account does not belong to signer");
        return Err(ProgramError::InvalidAccountData);
    }

    if user_balance.market != *market_info.key {
        msg!("User balance account does not belong to this market");
        return Err(ProgramError::InvalidAccountData);
    }

    let (market_pda, _) = Pubkey::find_program_address(
        &[
            b"market",
            market_state.base_mint.as_ref(),
            market_state.quote_mint.as_ref(),
        ],
        program_id,
    );

    if *market_info.key != market_pda {
        msg!("Invalid market account");
        return Err(ProgramError::InvalidAccountData);
    }

    let bids_seeds = &[b"bids", market_pda.as_ref()];
    let (bids_pda, _bids_bump) = Pubkey::find_program_address(bids_seeds, program_id);

    if bids_info.key != &bids_pda {
        msg!("Invalid bids account. Expected PDA: {}", bids_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    let asks_seeds = &[b"asks", market_pda.as_ref()];
    let (asks_pda, _asks_bump) = Pubkey::find_program_address(asks_seeds, program_id);

    if asks_info.key != &asks_pda {
        msg!("Invalid asks account. Expected PDA: {}", asks_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    let market_events_seeds = &[b"events", market_pda.as_ref()];
    let (market_events_pda, _events_bump) =
        Pubkey::find_program_address(market_events_seeds, program_id);

    if market_events_info.key != &market_events_pda {
        msg!(
            "Invalid market events account. Expected PDA: {}",
            market_events_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize orderbooks
    msg!("Deserializing bids...");
    let bids_data = bids_info.data.borrow();
    let min_size = 32 + 1 + 4 + 8; // 45 bytes for empty OrderBook
    let mut bids = if bids_data.len() >= min_size {
        let orders_len = u32::from_le_bytes([
            bids_data[33], bids_data[34], bids_data[35], bids_data[36]
        ]);
        let actual_size = min_size + (orders_len as usize * 105);
        let actual_data = &bids_data[0..actual_size];
        OrderBook::try_from_slice(actual_data)?
    } else {
        return Err(ProgramError::InvalidAccountData);
    };
    drop(bids_data);

    msg!("Deserializing asks...");
    let asks_data = asks_info.data.borrow();
    let mut asks = if asks_data.len() >= min_size {
        let orders_len = u32::from_le_bytes([
            asks_data[33], asks_data[34], asks_data[35], asks_data[36]
        ]);
        let actual_size = min_size + (orders_len as usize * 105);
        let actual_data = &asks_data[0..actual_size];
        OrderBook::try_from_slice(actual_data)?
    } else {
        return Err(ProgramError::InvalidAccountData);
    };
    drop(asks_data);

    // Deserialize market events
    msg!("Deserializing market events...");
    let events_data = market_events_info.data.borrow();
    let events_min_size = 32 + 8 + 8 + 8 + 8 + 4; // 68 bytes for empty MarketEvents
    let mut market_events = if events_data.len() >= events_min_size {
        let events_len = u32::from_le_bytes([
            events_data[64], events_data[65], events_data[66], events_data[67]
        ]);
        let actual_size = events_min_size + (events_len as usize * 98);
        let actual_data = &events_data[0..actual_size];
        MarketEvents::try_from_slice(actual_data)?
    } else {
        return Err(ProgramError::InvalidAccountData);
    };
    drop(events_data);

    let clock = Clock::from_account_info(clock_sysvar_info)?;

    // search order 
    let mut order_found = false;
    let mut cancelled_order_price = 0u64;
    let mut cancelled_order_quantity = 0u64;
    let mut cancelled_order_filled_quantity = 0u64;
    let mut cancelled_order_side = crate::state::Side::Buy;

    // bids
    let mut removed_from_bids = false;
    for i in 0..bids.orders.len() {
        if bids.orders[i].order_id == order_id && bids.orders[i].owner == *user_info.key {
            cancelled_order_price = bids.orders[i].price;
            cancelled_order_quantity = bids.orders[i].quantity;
            cancelled_order_filled_quantity = bids.orders[i].filled_quantity;
            cancelled_order_side = bids.orders[i].side;

            let remaining_quantity = bids.orders[i].quantity - bids.orders[i].filled_quantity;
            let locked_quote = (remaining_quantity * bids.orders[i].price) / 1_000_000_000;
            
            user_balance.locked_quote_balance -= locked_quote;
            user_balance.available_quote_balance += locked_quote;

            order_found = true;
            removed_from_bids = true;

            msg!(
                "Cancelled buy order {} with remaining quantity {} at price {}",
                order_id,
                remaining_quantity,
                bids.orders[i].price
            );
            break;
        }
    }

    if removed_from_bids {
        let mut write_index = 0;
        for read_index in 0..bids.orders.len() {
            if bids.orders[read_index].order_id == order_id 
                && bids.orders[read_index].owner == *user_info.key {
                // Skip this order (effectively removing it)
                continue;
            }
            if write_index != read_index {
                bids.orders[write_index] = bids.orders[read_index];
            }
            write_index += 1;
        }
        bids.orders.truncate(write_index);
        bids.active_orders_count -= 1;
    }

    let mut removed_from_asks = false;
    if !order_found {
        for i in 0..asks.orders.len() {
            if asks.orders[i].order_id == order_id && asks.orders[i].owner == *user_info.key {
                cancelled_order_price = asks.orders[i].price;
                cancelled_order_quantity = asks.orders[i].quantity;
                cancelled_order_filled_quantity = asks.orders[i].filled_quantity;
                cancelled_order_side = asks.orders[i].side;

                let remaining_quantity = asks.orders[i].quantity - asks.orders[i].filled_quantity;
                
                user_balance.locked_base_balance -= remaining_quantity;
                user_balance.available_base_balance += remaining_quantity;

                order_found = true;
                removed_from_asks = true;

                msg!(
                    "Cancelled sell order {} with remaining quantity {} at price {}",
                    order_id,
                    remaining_quantity,
                    asks.orders[i].price
                );
                break;
            }
        }

        if removed_from_asks {
            let mut write_index = 0;
            for read_index in 0..asks.orders.len() {
                if asks.orders[read_index].order_id == order_id 
                    && asks.orders[read_index].owner == *user_info.key {
                    continue;
                }
                if write_index != read_index {
                    asks.orders[write_index] = asks.orders[read_index];
                }
                write_index += 1;
            }
            asks.orders.truncate(write_index);
            asks.active_orders_count -= 1;
        }
    }

    if !order_found {
        msg!("Order {} not found or not owned by user", order_id);
        return Err(ProgramError::Custom(3)); // Order not found
    }

    let cancel_event = Event {
        event_type: EventType::Out,
        maker: *user_info.key,
        taker: Pubkey::default(), // No taker for cancel events
        maker_order_id: order_id,
        quantity: cancelled_order_quantity - cancelled_order_filled_quantity,
        price: cancelled_order_price,
        timestamp: clock.unix_timestamp,
        side: cancelled_order_side,
    };
    market_events.add_event(cancel_event)?;

    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;
    bids.serialize(&mut *bids_info.data.borrow_mut())?;
    asks.serialize(&mut *asks_info.data.borrow_mut())?;
    market_events.serialize(&mut *market_events_info.data.borrow_mut())?;
    market_state.serialize(&mut *market_info.data.borrow_mut())?;

    msg!("Order {} cancelled successfully", order_id);
    Ok(())
} 