use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

mod instructions;

mod state;
use instructions::{
    process_consume_events,
    process_create_acc_and_deposit_base_tokens,
    process_create_acc_and_deposit_quote_tokens,
    process_initialize_market,
    process_place_order,
    process_settle_balance, 
    process_cancel_order,
};
use state::Side;

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum Instruction {
    InitializeMarket {
        min_order_size: u64,
        tick_size: u64,
    },
    DepositQuoteTokens {
        quantity: u64,
    },
    DepositBaseTokens {
        quantity: u64,
    },
    PlaceOrder {
        side: Side,
        price: u64,
        quantity: u64,
    },
    ConsumeEvents,
    SettleBalance,
    CancelOrder {
        order_id: u64,
    },
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

        Instruction::DepositQuoteTokens { quantity } => {
            msg!("Instruction: Deposit Quote Tokens");
            process_create_acc_and_deposit_quote_tokens(program_id, accounts, quantity)
        }
        Instruction::DepositBaseTokens { quantity } => {
            msg!("Instruction: Deposit Base Tokens");
            process_create_acc_and_deposit_base_tokens(program_id, accounts, quantity)
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
              Instruction::CancelOrder { order_id } => {
                    msg!("Instruction: Cancel Order");
                    process_cancel_order(program_id, accounts, order_id)
                }
    }
}
