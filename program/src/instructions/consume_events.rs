use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::collections::HashMap;

use crate::state::{EventType, MarketEvents, MarketState, Side, UserBalance};

const MAX_EVENTS_TO_CONSUME: usize = 7;

pub fn process_consume_events(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let consume_events_authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_events_info = next_account_info(account_info_iter)?;

    if !consume_events_authority_info.is_signer {
        msg!("Consume events authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
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

    if market_events_info.owner != program_id {
        msg!("Market events account must be owned by this program");
        return Err(ProgramError::InvalidAccountData);
    }

    if market_state.consume_events_authority != *consume_events_authority_info.key {
        msg!("Invalid consume events authority");
        return Err(ProgramError::InvalidAccountData);
    }

    let remaining_accounts: Vec<&AccountInfo> = account_info_iter.collect();
    let mut events_data = market_events_info.data.borrow_mut();
    let market_events: &mut MarketEvents = bytemuck::from_bytes_mut(&mut events_data);

    let mut consumed_count: usize = 0;

    let mut balance_accounts: HashMap<Pubkey, &AccountInfo> = HashMap::new();
    for account_info in &remaining_accounts {
        balance_accounts.insert(*account_info.key, account_info);
    }

    msg!(
        "Starting event consumption. Events to process: {}",
        market_events.events_to_process
    );

    for i in 0..(market_events.events_to_process as usize) {
        if i >= market_events.events.len() {
            break;
        }
        if consumed_count >= MAX_EVENTS_TO_CONSUME {
            msg!("Maximum event limit reached: {}", MAX_EVENTS_TO_CONSUME);
            break;
        }

        let event = &market_events.events[i];
        
        let event_maker = event.maker;
        let event_taker = event.taker;
        let event_type = event.event_type;
        let event_side = event.side;
        let event_quantity = event.quantity;
        let event_price = event.price;

        // Skip empty/removed events
        if event_maker == Pubkey::default() && event_taker == Pubkey::default() {
            msg!("Skipping empty event at index {}", i);
            continue;
        }

        msg!(
            "Processing event {}: {} {} {} at {} price",
            i,
            event_type as u8,
            event_quantity,
            event_side as u8,
            event_price
        );

        let quote_amount = (event_quantity * event_price) / 1_000_000_000;

        match event_type {
            EventType::Fill => {
                // maker == taker ,self-trade
                if event_maker == event_taker {
                    msg!("Self-trade detected: maker == taker");
                    
                    let (user_balance_pda, _) = Pubkey::find_program_address(
                        &[
                            b"user_balance",
                            event_maker.as_ref(),
                            market_info.key.as_ref(),
                        ],
                        program_id,
                    );

                    if let Some(user_balance_info) = balance_accounts.get(&user_balance_pda) {
                        let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;
                        
                        if user_balance.owner == event_maker {
                            if event_side == Side::Buy {
                                user_balance.locked_quote_balance -= quote_amount;
                                user_balance.available_quote_balance += quote_amount;
                            } else {
                                user_balance.locked_base_balance -= event_quantity;
                                user_balance.available_base_balance += event_quantity;
                            }
                            
                            user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;
                            msg!("Self-trade balance updated - unlocked funds with no net change");
                        }
                    } else {
                        msg!("User balance account not found for self-trade");
                    }
                } else {
                    //normal trade
                    let (maker_balance_pda, _) = Pubkey::find_program_address(
                        &[
                            b"user_balance",
                            event_maker.as_ref(),
                            market_info.key.as_ref(),
                        ],
                        program_id,
                    );

                    if let Some(maker_balance_info) = balance_accounts.get(&maker_balance_pda) {
                        let mut maker_balance = UserBalance::try_from_slice(&maker_balance_info.data.borrow())?;

                        if maker_balance.owner == event_maker {
                            if event_side == Side::Buy {
                                // Taker is buying, so maker is selling
                                maker_balance.locked_base_balance -= event_quantity;
                                maker_balance.pending_quote_balance += quote_amount;
                                msg!("Maker sold: -{} base locked, +{} quote pending", event_quantity, quote_amount);
                            } else {
                                // Taker is selling, so maker is buying
                                maker_balance.locked_quote_balance -= quote_amount;
                                maker_balance.pending_base_balance += event_quantity;
                                msg!("Maker bought: -{} quote locked, +{} base pending", quote_amount, event_quantity);
                            }
                            
                            maker_balance.serialize(&mut *maker_balance_info.data.borrow_mut())?;
                            msg!("Maker balance updated");
                        }
                    } else {
                        msg!("Maker balance account not found, skipping maker settlement");
                    }

                    // Process taker's balance
                    let (taker_balance_pda, _) = Pubkey::find_program_address(
                        &[
                            b"user_balance",
                            event_taker.as_ref(),
                            market_info.key.as_ref(),
                        ],
                        program_id,
                    );

                    if let Some(taker_balance_info) = balance_accounts.get(&taker_balance_pda) {
                        let mut taker_balance = UserBalance::try_from_slice(&taker_balance_info.data.borrow())?;

                        if taker_balance.owner == event_taker {
                            if event_side == Side::Buy {
                                // Taker is buying
                                taker_balance.locked_quote_balance -= quote_amount;
                                taker_balance.pending_base_balance += event_quantity;
                                msg!("Taker bought: -{} quote locked, +{} base pending", quote_amount, event_quantity);
                            } else {
                                // Taker is selling
                                taker_balance.locked_base_balance -= event_quantity;
                                taker_balance.pending_quote_balance += quote_amount;
                                msg!("Taker sold: -{} base locked, +{} quote pending", event_quantity, quote_amount);
                            }

                            taker_balance.serialize(&mut *taker_balance_info.data.borrow_mut())?;
                            msg!("Taker balance updated");
                        }
                    } else {
                        msg!("Taker balance account not found, skipping taker settlement");
                    }
                }
            }
            EventType::Out => {
                // only for makers
                let (maker_balance_pda, _) = Pubkey::find_program_address(
                    &[
                        b"user_balance",
                        event_maker.as_ref(),
                        market_info.key.as_ref(),
                    ],
                    program_id,
                );

                if let Some(maker_balance_info) = balance_accounts.get(&maker_balance_pda) {
                    let mut maker_balance = UserBalance::try_from_slice(&maker_balance_info.data.borrow())?;

                    if maker_balance.owner == event_maker {
                        if event_side == Side::Buy {
                            // cancelled buy order,unlock quote tokens
                            maker_balance.locked_quote_balance -= quote_amount;
                            maker_balance.available_quote_balance += quote_amount;
                            msg!("Buy order cancelled: unlocked {} quote", quote_amount);
                        } else {
                            // cancelled sell order,unlock base tokens
                            maker_balance.locked_base_balance -= event_quantity;
                            maker_balance.available_base_balance += event_quantity;
                            msg!("Sell order cancelled: unlocked {} base", event_quantity);
                        }
                        
                        maker_balance.serialize(&mut *maker_balance_info.data.borrow_mut())?;
                        msg!("Cancelled order balance updated");
                    }
                } else {
                    msg!("Maker balance account not found for cancelled order");
                }
            }
        }

        consumed_count += 1;
        msg!("Event {} consumed successfully", i);
    }
    market_events.events_to_process = market_events.events_to_process.saturating_sub(consumed_count as u64);

    msg!(
        "Successfully consumed {} events. Remaining events: {}",
        consumed_count,
        market_events.events_to_process
    );

    Ok(())
}
