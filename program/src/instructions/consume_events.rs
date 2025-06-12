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
    
    // manual deserialization
    let events_data = market_events_info.data.borrow();
    
    // MarketEvents: market(32) + head(8) + count(8) + seq_num(8) + events_to_process(8) + vec_len(4) + events
    let min_size = 32 + 8 + 8 + 8 + 8 + 4; // 68 bytes for empty MarketEvents
    let mut market_events = if events_data.len() >= min_size {
        let events_len = u32::from_le_bytes([
            events_data[64], events_data[65], events_data[66], events_data[67]
        ]);
        msg!("Events vector length: {}", events_len);
        
        // Each event is 98 bytes
        let actual_size = min_size + (events_len as usize * 98);
        msg!("Market events calculated actual data size: {}", actual_size);
        
        let actual_data = &events_data[0..actual_size];
        let events = MarketEvents::try_from_slice(actual_data)?;
        msg!("Market events deserialized successfully");
        events
    } else {
        return Err(ProgramError::InvalidAccountData);
    };
    drop(events_data);

    if market_state.consume_events_authority != *consume_events_authority_info.key {
        msg!("Invalid consume events authority");
        return Err(ProgramError::InvalidAccountData);
    }

    // collect remaining pda's for lookup
    let remaining_accounts: Vec<&AccountInfo> = account_info_iter.collect();

    let mut consumed_count: usize = 0;

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

        // skip empty/removed events
        if event.maker == Pubkey::default() && event.taker == Pubkey::default() {
            msg!("Skipping empty event at index {}", i);
            continue;
        }

        msg!(
            "Processing event {}: {} {} {} at {} price",
            i,
            event.event_type as u8,
            event.quantity,
            event.side as u8,
            event.price
        );

        let (maker_balance_pda, _) = Pubkey::find_program_address(
            &[
                b"user_balance",
                event.maker.as_ref(),
                market_info.key.as_ref(),
            ],
            program_id,
        );

        if let Some(maker_balance_info) = remaining_accounts
            .iter()
            .find(|acc| *acc.key == maker_balance_pda)
        {
            let mut maker_balance = UserBalance::try_from_slice(&maker_balance_info.data.borrow())?;

            if maker_balance.owner == event.maker {
                match event.event_type {
                    EventType::Fill => {
                        let quote_amount = (event.quantity * event.price) / 1_000_000_000;
                        if event.side == Side::Sell {
                            // maker = buyer
                            maker_balance.locked_quote_balance -= quote_amount;
                            maker_balance.pending_base_balance += event.quantity;
                        } else {
                            // maker = seller
                            maker_balance.locked_base_balance -= event.quantity;
                            maker_balance.pending_quote_balance += quote_amount;
                        }
                    }
                    EventType::Out => {
                        let quote_amount = (event.quantity * event.price) / 1_000_000_000;
                        if event.side == Side::Buy {
                            maker_balance.locked_quote_balance -= quote_amount;
                            maker_balance.available_quote_balance += quote_amount;
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

        if event.event_type == EventType::Fill {
            let (taker_balance_pda, _) = Pubkey::find_program_address(
                &[
                    b"user_balance",
                    event.taker.as_ref(),
                    market_info.key.as_ref(),
                ],
                program_id,
            );

            if let Some(taker_balance_info) = remaining_accounts
                .iter()
                .find(|acc| *acc.key == taker_balance_pda)
            {
                let mut taker_balance =
                    UserBalance::try_from_slice(&taker_balance_info.data.borrow())?;

              if taker_balance.owner == event.taker {
                    let quote_amount = (event.quantity * event.price) / 1_000_000_000;
                    if event.side == Side::Buy {
                        taker_balance.locked_quote_balance -= quote_amount;
                        taker_balance.pending_base_balance += event.quantity;
                        msg!(
                            "Taker bought: +{} base pending, -{} quote",
                            event.quantity,
                            quote_amount
                        );
                    } else {
                        taker_balance.locked_base_balance -= event.quantity;
                        taker_balance.pending_quote_balance += quote_amount;
                        msg!(
                            "Taker sold: -{} base, +{} quote pending",
                            event.quantity,
                            quote_amount
                        );
                    }

                    taker_balance.serialize(&mut *taker_balance_info.data.borrow_mut())?;
                    msg!("Taker balance updated");
                }
            } else {
                msg!("Taker balance account not found, skipping taker settlement");
            }
        }

        consumed_count += 1;
        msg!("Event {} consumed successfully", i);
    }

    // note: On-chain, vec.remove() doesn't actually deallocate memory, so we manually truncate.
    //vec.remove shifts vec for each iteration so we shiift elements to front and truncate to reduce compute
    if consumed_count > 0 {
        msg!(
            "Compacting event array. Consumed: {}, Remaining: {}",
            consumed_count,
            market_events.events_to_process - consumed_count as u64
        );

        let remaining_events = market_events.events_to_process as usize - consumed_count;

        for i in 0..remaining_events {
            let source_index = i + consumed_count;
            if source_index < market_events.events.len() {
                market_events.events[i] = market_events.events[source_index];
            }
        }

        market_events.events_to_process -= consumed_count as u64;

        market_events.events.truncate(remaining_events);

        msg!(
            "Array compaction completed. New size: {}",
            market_events.events.len()
        );
    }

    market_events.serialize(&mut *market_events_info.data.borrow_mut())?;

    msg!(
        "Successfully consumed {} events. Remaining events: {}",
        consumed_count,
        market_events.events_to_process
    );

    Ok(())
}
