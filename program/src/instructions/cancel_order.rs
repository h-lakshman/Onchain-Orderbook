use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

use crate::state::{Event, EventType, MarketEvents, MarketState, OrderBook, Side, UserBalance};

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

    if bids_info.owner != program_id {
        msg!("Bids account must be owned by this program");
        return Err(ProgramError::InvalidAccountData);
    }

    if asks_info.owner != program_id {
        msg!("Asks account must be owned by this program");
        return Err(ProgramError::InvalidAccountData);
    }

    if market_events_info.owner != program_id {
        msg!("Market events account must be owned by this program");
        return Err(ProgramError::InvalidAccountData);
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

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    
    let mut bids_data = bids_info.data.borrow_mut();
    let mut asks_data = asks_info.data.borrow_mut();
    let mut market_events_data = market_events_info.data.borrow_mut();
    
    let bids: &mut OrderBook = bytemuck::from_bytes_mut(&mut bids_data);
    let asks: &mut OrderBook = bytemuck::from_bytes_mut(&mut asks_data);
    let market_events: &mut MarketEvents = bytemuck::from_bytes_mut(&mut market_events_data);

    let mut order_found = false;
    let mut cancelled_order_price = 0u64;
    let mut cancelled_order_quantity = 0u64;
    let mut cancelled_order_filled_quantity = 0u64;
    let mut cancelled_order_side = Side::Buy;

    let mut order_index_to_remove: Option<usize> = None;

    for i in 0..(bids.active_orders_count as usize) {
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
            order_index_to_remove = Some(i);

            let order_price = bids.orders[i].price;
            msg!(
                "Cancelled buy order {} with remaining quantity {} at price {}",
                order_id,
                remaining_quantity,
                order_price
            );
            break;
        }
    }

    if let Some(index) = order_index_to_remove {
        bids.remove_order(index)?;
    }

    if !order_found {
        for i in 0..(asks.active_orders_count as usize) {
            if asks.orders[i].order_id == order_id && asks.orders[i].owner == *user_info.key {
                cancelled_order_price = asks.orders[i].price;
                cancelled_order_quantity = asks.orders[i].quantity;
                cancelled_order_filled_quantity = asks.orders[i].filled_quantity;
                cancelled_order_side = asks.orders[i].side;

                let remaining_quantity = asks.orders[i].quantity - asks.orders[i].filled_quantity;

                user_balance.locked_base_balance -= remaining_quantity;
                user_balance.available_base_balance += remaining_quantity;

                order_found = true;

                let order_price = asks.orders[i].price;
                msg!(
                    "Cancelled sell order {} with remaining quantity {} at price {}",
                    order_id,
                    remaining_quantity,
                    order_price
                );

                asks.remove_order(i)?;
                break;
            }
        }
    }

    if !order_found {
        msg!("Order {} not found or not owned by user", order_id);
        return Err(ProgramError::Custom(3));
    }

    let cancel_event = Event {
        event_type: EventType::Out,
        maker: *user_info.key,
        taker: Pubkey::default(),
        maker_order_id: order_id,
        quantity: cancelled_order_quantity - cancelled_order_filled_quantity,
        price: cancelled_order_price,
        timestamp: clock.unix_timestamp,
        side: cancelled_order_side,
    };
    market_events.add_event(cancel_event)?;

    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;

    msg!("Order {} cancelled successfully", order_id);
    Ok(())
}
