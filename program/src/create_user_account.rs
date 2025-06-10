use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo}, 
    entrypoint::ProgramResult, 
    msg, 
    program::{invoke, invoke_signed}, 
    program_error::ProgramError, 
    pubkey::Pubkey, 
    rent::Rent, 
    system_instruction, 
    system_program, 
    sysvar::Sysvar 
};
use spl_token::instruction as token_instruction;

use crate::state::{UserBalance};

pub fn process_create_update_user_balance_account(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    onramp_quantity: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let new_user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let user_quote_token_account_info = next_account_info(account_info_iter)?;
    let quote_vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    if !user_info.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !system_program::check_id(system_program_info.key) {
        msg!("Invalid system program");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_token::check_id(token_program_info.key) {
        msg!("Invalid token program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let new_user_account_seeds = &[b"user_balance", user_info.key.as_ref(), market_info.key.as_ref()];
    let (new_user_account_pda, new_user_account_bump) = 
        Pubkey::find_program_address(new_user_account_seeds, program_id);

    if new_user_balance_info.key != &new_user_account_pda {
        msg!(
            "Invalid new user account. Expected PDA: {}",
            new_user_account_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::from_account_info(rent_info)?;
    let account_exists = new_user_balance_info.lamports() > 0 && 
                        new_user_balance_info.data_len() == UserBalance::LEN;

    if !account_exists {
        msg!("Creating user balance account with {} bytes", UserBalance::LEN);
        let new_user_account_rent = rent.minimum_balance(UserBalance::LEN);
        let create_new_user_account_ix = system_instruction::create_account(
            user_info.key,
            &new_user_account_pda,
            new_user_account_rent,
            UserBalance::LEN as u64,
            program_id,
        );

        invoke_signed(
            &create_new_user_account_ix,
            &[
                user_info.clone(),
                new_user_balance_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"user_balance", user_info.key.as_ref(), market_info.key.as_ref(), &[new_user_account_bump]]],
        )?;

        let user_balance_account_data = UserBalance {
            owner: *user_info.key,
            market: *market_info.key,
            available_base_balance: 0,
            available_quote_balance: 0,
            locked_base_balance: 0,
            locked_quote_balance: 0,
            pending_base_balance: 0,
            pending_quote_balance: 0,
        };

        let mut data = new_user_balance_info.data.borrow_mut();
        user_balance_account_data.serialize(&mut &mut data[..])?;
        msg!("User balance account created successfully!");
    } else {
        msg!("User balance account already exists, updating...");
    }

    if onramp_quantity > 0 {
        msg!("Processing onramp of {} tokens", onramp_quantity);

        // transfer tokens from user's token account to quote vault
        let transfer_ix = token_instruction::transfer(
            token_program_info.key,
            user_quote_token_account_info.key,
            quote_vault_info.key,
            user_info.key,
            &[],
            onramp_quantity,
        )?;

        invoke(
            &transfer_ix,
            &[
                user_quote_token_account_info.clone(),
                quote_vault_info.clone(),
                user_info.clone(),
                token_program_info.clone(),
            ],
        )?;

        let mut user_balance = UserBalance::try_from_slice(&new_user_balance_info.data.borrow())?;
        user_balance.available_quote_balance += onramp_quantity;
        user_balance.serialize(&mut *new_user_balance_info.data.borrow_mut())?;

        msg!("Successfully deposited {} tokens to user balance", onramp_quantity);
        msg!("New available quote balance: {}", user_balance.available_quote_balance);
    }

    msg!("User PDA: {}", new_user_account_pda);
    msg!("Owner: {}", user_info.key);
    msg!("Market: {}", market_info.key);
    
    Ok(())
}