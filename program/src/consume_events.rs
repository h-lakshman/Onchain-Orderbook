use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::state::{EventType, MarketEvents, MarketState, Side, UserBalance};

const MAX_EVENTS_TO_CONSUME: usize = 7;

pub fn process_consume_events(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let consume_events_authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_events_info = next_account_info(account_info_iter)?;

    // Verify authority is signer
    if !consume_events_authority_info.is_signer {
        msg!("Consume events authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut market_events = MarketEvents::try_from_slice(&market_events_info.data.borrow())?;

    // Verify the authority
    if market_state.consume_events_authority != *consume_events_authority_info.key {
        msg!("Invalid consume events authority");
        return Err(ProgramError::InvalidAccountData);
    }

    // Collect remaining accounts for PDA lookup
    let remaining_accounts: Vec<&AccountInfo> = account_info_iter.collect();
    
    let mut consumed_count: usize = 0;

    msg!("Starting event consumption. Events to process: {}", market_events.events_to_process);

    for i in 0..(market_events.events_to_process as usize) {
        if i >= market_events.events.len() {
            break;
        }

        if consumed_count >= MAX_EVENTS_TO_CONSUME {
            msg!("Maximum event limit reached: {}", MAX_EVENTS_TO_CONSUME);
            break;
        }

        let event = &market_events.events[i];

        // Skip empty/removed events (in case of sparse array)
        if event.maker == Pubkey::default() && event.taker == Pubkey::default() {
            msg!("Skipping empty event at index {}", i);
            continue;
        }

        msg!("Processing event {}: {} {} {} at {} price", 
             i, event.event_type as u8, event.quantity, event.side as u8, event.price);

        let mut event_consumed = false;

        // Process MAKER settlement
        let (maker_balance_pda, _) = Pubkey::find_program_address(
            &[b"user_balance", event.maker.as_ref(), market_info.key.as_ref()],
            program_id,
        );

        if let Some(maker_balance_info) = remaining_accounts.iter().find(|acc| *acc.key == maker_balance_pda) {
            let mut maker_balance = UserBalance::try_from_slice(&maker_balance_info.data.borrow())?;

            if maker_balance.owner == event.maker {
                match event.event_type {
                    EventType::Fill => {
                        if event.side == Side::Buy {
                            // Taker bought, so maker sold
                            maker_balance.locked_base_balance -= event.quantity;
                            maker_balance.pending_quote_balance += event.quantity * event.price;
                            msg!("Maker sold: -{} base, +{} quote pending", event.quantity, event.quantity * event.price);
                        } else {
                            // Taker sold, so maker bought
                            maker_balance.locked_quote_balance -= event.quantity * event.price;
                            maker_balance.pending_base_balance += event.quantity;
                            msg!("Maker bought: +{} base pending, -{} quote", event.quantity, event.quantity * event.price);
                        }
                    }
                    EventType::Out => {
                        if event.side == Side::Buy {
                            maker_balance.locked_quote_balance -= event.quantity * event.price;
                            maker_balance.available_quote_balance += event.quantity * event.price;
                        } else {
                            maker_balance.locked_base_balance -= event.quantity;
                            maker_balance.available_base_balance += event.quantity;
                        }
                    }
                }
                maker_balance.serialize(&mut *maker_balance_info.data.borrow_mut())?;
                msg!("Maker balance updated");
            }
        } else {
            msg!("Maker balance account not found, skipping maker settlement");
        }

        // Process TAKER settlement (only for Fill events)
        if event.event_type == EventType::Fill {
            let (taker_balance_pda, _) = Pubkey::find_program_address(
                &[b"user_balance", event.taker.as_ref(), market_info.key.as_ref()],
                program_id,
            );

            if let Some(taker_balance_info) = remaining_accounts.iter().find(|acc| *acc.key == taker_balance_pda) {
                let mut taker_balance = UserBalance::try_from_slice(&taker_balance_info.data.borrow())?;

                if taker_balance.owner == event.taker {
                    if event.side == Side::Buy {
                        // Taker bought
                        taker_balance.locked_quote_balance -= event.quantity * event.price;
                        taker_balance.pending_base_balance += event.quantity;
                        msg!("Taker bought: +{} base pending, -{} quote", event.quantity, event.quantity * event.price);
                    } else {
                        // Taker sold
                        taker_balance.locked_base_balance -= event.quantity;
                        taker_balance.pending_quote_balance += event.quantity * event.price;
                        msg!("Taker sold: -{} base, +{} quote pending", event.quantity, event.quantity * event.price);
                    }

                    taker_balance.serialize(&mut *taker_balance_info.data.borrow_mut())?;
                    msg!("Taker balance updated");
                }
            } else {
                msg!("Taker balance account not found, skipping taker settlement");
            }
        }

        event_consumed = true;
        consumed_count += 1;
        msg!("Event {} consumed successfully", i);
    }

    // Array compaction - move unprocessed events to the front
    if consumed_count > 0 {
        msg!("Compacting event array. Consumed: {}, Remaining: {}", 
             consumed_count, market_events.events_to_process - consumed_count as u64);

        // Remove processed events from the beginning
        let remaining_events = market_events.events_to_process as usize - consumed_count;
        
        // Move unprocessed events to the front
        for i in 0..remaining_events {
            let source_index = i + consumed_count;
            if source_index < market_events.events.len() {
                market_events.events[i] = market_events.events[source_index];
            }
        }

        // Update the events_to_process counter
        market_events.events_to_process -= consumed_count as u64;

        // Truncate the events vector to remove processed events
        market_events.events.truncate(remaining_events);
        
        msg!("Array compaction completed. New size: {}", market_events.events.len());
    }

    // Serialize updated market events
    market_events.serialize(&mut *market_events_info.data.borrow_mut())?;

    msg!("Successfully consumed {} events. Remaining events: {}", 
         consumed_count, market_events.events_to_process);

    Ok(())
}