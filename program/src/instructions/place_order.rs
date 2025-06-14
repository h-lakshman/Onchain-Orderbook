use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};
use spl_token::instruction as token_instruction;

use crate::state::{
    Event, EventType, MarketEvents, MarketState, Order, OrderBook, Side, UserBalance,
};

pub fn process_place_order(
    program_id: &Pubkey,
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
    let user_base_token_info = next_account_info(account_info_iter)?;
    let user_quote_token_info = next_account_info(account_info_iter)?;
    let market_base_vault_info = next_account_info(account_info_iter)?;
    let market_quote_vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let clock_sysvar_info = next_account_info(account_info_iter)?;

    if !spl_token::check_id(token_program_info.key) {
        msg!("Invalid token program");
        return Err(ProgramError::IncorrectProgramId);
    }

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

    let mut market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;

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
    if user_balance.owner != *user_info.key {
        msg!("User balance account does not belong to signer");
        return Err(ProgramError::InvalidAccountData);
    }

    if user_balance.market != *market_info.key {
        msg!("User balance account does not belong to this market");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("Checks on user quote token acc");
    {
        let user_quote_token_data = user_quote_token_info.data.borrow();
        if user_quote_token_data.len() < 32 {
            msg!("Invalid user quote token account data");
            return Err(ProgramError::InvalidAccountData);
        }
        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&user_quote_token_data[0..32]);
        let user_token_mint = Pubkey::new_from_array(mint_bytes);
        if user_token_mint != market_state.quote_mint {
            msg!(
                "User quote token account mint mismatch. Expected: {}, Got: {}",
                market_state.quote_mint,
                user_token_mint
            );
            return Err(ProgramError::InvalidAccountData);
        }
    }

    msg!("Checks on user base token acc");
    {
        let user_base_token_data = user_base_token_info.data.borrow();
        if user_base_token_data.len() < 32 {
            msg!("Invalid user base token account data");
            return Err(ProgramError::InvalidAccountData);
        }
        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&user_base_token_data[0..32]);
        let user_token_mint = Pubkey::new_from_array(mint_bytes);
        if user_token_mint != market_state.base_mint {
            msg!(
                "User base token account mint mismatch. Expected: {}, Got: {}",
                market_state.base_mint,
                user_token_mint
            );
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let base_vault_seeds = &[b"base_vault", market_pda.as_ref()];
    let (base_vault_pda, _base_vault_bump) =
        Pubkey::find_program_address(base_vault_seeds, program_id);

    if market_state.base_vault != base_vault_pda {
        msg!(
            "Invalid base vault account. Expected PDA: {}",
            base_vault_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let quote_vault_seeds = &[b"quote_vault", market_pda.as_ref()];
    let (quote_vault_pda, _quote_vault_bump) =
        Pubkey::find_program_address(quote_vault_seeds, program_id);

    if market_state.quote_vault != quote_vault_pda {
        msg!(
            "Invalid quote vault account. Expected PDA: {}",
            quote_vault_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let mut bids_data = bids_info.data.borrow_mut();
    let mut asks_data = asks_info.data.borrow_mut();
    let mut bids: &mut OrderBook = bytemuck::from_bytes_mut(&mut bids_data);
    let mut asks: &mut OrderBook = bytemuck::from_bytes_mut(&mut asks_data);

    let (taker_book, maker_book) = if side == Side::Buy {
        (&mut bids, &mut asks)
    } else {
        (&mut asks, &mut bids)
    };

    let required_base = if side == Side::Sell { quantity } else { 0 };
    let required_quote = if side == Side::Buy {
        (quantity * price) / 1_000_000_000
    } else {
        0
    };

    if user_balance.available_base_balance < required_base
        || user_balance.available_quote_balance < required_quote
    {
        msg!("Insufficient funds to place order");
        return Err(ProgramError::InsufficientFunds);
    }

    if side == Side::Buy {
        msg!(
            "Transferring {} quote tokens to market vault",
            required_quote
        );

        let transfer_quote_ix = token_instruction::transfer(
            token_program_info.key,
            user_quote_token_info.key,
            market_quote_vault_info.key,
            user_info.key,
            &[],
            required_quote,
        )?;

        invoke(
            &transfer_quote_ix,
            &[
                user_quote_token_info.clone(),
                market_quote_vault_info.clone(),
                user_info.clone(),
                token_program_info.clone(),
            ],
        )?;

        msg!("Quote tokens transferred successfully");
    } else {
        msg!("Transferring {} base tokens to market vault", required_base);

        let transfer_base_ix = token_instruction::transfer(
            token_program_info.key,
            user_base_token_info.key,
            market_base_vault_info.key,
            user_info.key,
            &[],
            required_base,
        )?;

        invoke(
            &transfer_base_ix,
            &[
                user_base_token_info.clone(),
                market_base_vault_info.clone(),
                user_info.clone(),
                token_program_info.clone(),
            ],
        )?;

        msg!("Base tokens transferred successfully");
    }

    user_balance.available_base_balance -= required_base;
    user_balance.locked_base_balance += required_base;
    user_balance.available_quote_balance -= required_quote;
    user_balance.locked_quote_balance += required_quote;

    let mut maker_events_data = market_events_info.data.borrow_mut();
    let maker_events: &mut MarketEvents = bytemuck::from_bytes_mut(&mut maker_events_data);
    let mut remaining_quantity = quantity;
    let mut orders_to_remove = Vec::new();

    for i in 0..maker_book.orders.len() {
        let maker_order = &mut maker_book.orders[i];
        if remaining_quantity == 0 {
            break;
        }

        let price_match = if side == Side::Buy {
            price >= maker_order.price
        } else {
            price <= maker_order.price
        };

        if price_match {
            let fill_quantity = std::cmp::min(
                remaining_quantity,
                maker_order.quantity - maker_order.filled_quantity,
            );

            if fill_quantity > 0 {
                maker_order.filled_quantity += fill_quantity;
                remaining_quantity -= fill_quantity;

                let maker_fill_event = Event {
                    event_type: EventType::Fill,
                    maker: maker_order.owner,
                    taker: *user_info.key,
                    maker_order_id: maker_order.order_id,
                    quantity: fill_quantity,
                    price: maker_order.price,
                    timestamp: clock.unix_timestamp,
                    side,
                };

                maker_events.add_event(maker_fill_event)?;

                let price = maker_order.price;
                msg!("Filled {} quantity at {} price", fill_quantity, price);

                if maker_order.filled_quantity == maker_order.quantity {
                    orders_to_remove.push(i);
                }
            }
        }
    }

    for &index in orders_to_remove.iter().rev() {
        maker_book.remove_order(index)?;
    }

    if remaining_quantity > 0 {
        let new_order = Order {
            order_id: market_state.next_order_id,
            owner: *user_info.key,
            market: *market_info.key,
            side,
            price,
            quantity: remaining_quantity,
            filled_quantity: 0,
            timestamp: clock.unix_timestamp,
        };
        taker_book.add_order(new_order)?;
        market_state.next_order_id += 1;

        msg!(
            "Added remaining order: {} quantity at {} price",
            remaining_quantity,
            price
        );
    } else {
        msg!("Order fully filled, no remaining quantity");
    }

    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;
    market_state.serialize(&mut *market_info.data.borrow_mut())?;

    msg!("Order placement completed successfully");
    Ok(())
}
