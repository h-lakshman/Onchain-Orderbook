use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

mod initialize_market;
mod create_user_account;
mod place_order;
mod consume_events;
mod settle_balance;
mod state;
use initialize_market::process_initialize_market;
use place_order::process_place_order;
use consume_events::process_consume_events;
use settle_balance::process_settle_balance;
use state::Side;

use crate::create_user_account::process_create_update_user_balance_account;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Instruction {
    InitializeMarket { 
        min_order_size: u64, 
        tick_size: u64 
    },
    CreateUpdateUserBalanceAccount { onramp_quantity: u64 },
    PlaceOrder {
        side: Side,
        price: u64,
        quantity: u64,
    },
    ConsumeEvents,
    SettleBalance,
}

entrypoint!(process_instruction);

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Orderbook program");

    let instruction = Instruction::try_from_slice(instruction_data)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match instruction {
        Instruction::InitializeMarket {
            min_order_size,
            tick_size,
        } => {
            msg!("Instruction: Initialize Market");
            process_initialize_market(program_id, accounts, min_order_size, tick_size)
        }
        Instruction::CreateUpdateUserBalanceAccount { onramp_quantity } => {
            msg!("Instruction: Create User Balance Account");
            process_create_update_user_balance_account(program_id, accounts, onramp_quantity)
        }
        Instruction::PlaceOrder {
            side,
            price,
            quantity,
        } => {
            msg!("Instruction: Place Order");
            process_place_order(program_id, accounts, side, price, quantity)
        }
        Instruction::ConsumeEvents => {
            msg!("Instruction: Consume Events");
            process_consume_events(program_id, accounts)
        }
        Instruction::SettleBalance => {
            msg!("Instruction: Settle Balance");
            process_settle_balance(program_id, accounts)
        }
    }
}
