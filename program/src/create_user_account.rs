use borsh::BorshSerialize;
use solana_program::{account_info::{next_account_info, AccountInfo}, entrypoint::ProgramResult, msg, program::invoke_signed, program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, system_program, sysvar::Sysvar };

use crate::state::{UserBalance};

pub fn process_create_user_balance_account(program_id:&Pubkey,accounts:&[AccountInfo]) -> ProgramResult {
    let account_info_iter =&mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let new_user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
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

    let new_user_account_seeds = &[b"balance",user_info.key.as_ref()];
    let (new_user_account_pda,new_user_account_bump)  =  Pubkey::find_program_address(new_user_account_seeds, program_id);

    if new_user_balance_info.key != &new_user_account_pda {
        msg!(
            "Invalid new user account. Expected PDA: {}",
            new_user_account_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }
    
    let rent = Rent::from_account_info(rent_info)?;
    
    if new_user_balance_info.lamports() > 0 && new_user_balance_info.data_len() == UserBalance::LEN {
        msg!("User balance account already exists for user: {}", user_info.key);
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    
    if new_user_balance_info.lamports() == 0 {
        msg!("Creating user balance account with {} bytes", UserBalance::LEN);
        let new_user_account_rent = rent.minimum_balance(UserBalance::LEN);
        let create_new_user_account_ix = system_instruction::create_account(
            user_info.key,
            &new_user_account_pda,
            new_user_account_rent,
            UserBalance::LEN as u64,
            program_id);

        invoke_signed(&create_new_user_account_ix,
            &[user_info.clone(),
            new_user_balance_info.clone(),
            system_program_info.clone()],
            &[&[b"balance",user_info.key.as_ref(),&[new_user_account_bump]]]
        )?;
    }

    let user_balance_account_data = UserBalance {
        owner:*user_info.key,
        market:*market_info.key,
        available_base_balance: 0,
        available_quote_balance: 0,
        locked_base_balance: 0,
        locked_quote_balance: 0,
        pending_base_balance: 0,
        pending_quote_balance: 0,
    };

    let mut data = new_user_balance_info.data.borrow_mut();
    let mut serialized_data= Vec::new();
    user_balance_account_data.serialize(&mut serialized_data)?;

    data[..serialized_data.len()].copy_from_slice(&serialized_data);

    msg!("User balance account created successfully!");
    msg!("User PDA: {}", new_user_account_pda);
    msg!("Owner: {}", user_info.key);
    msg!("Market: {}", market_info.key);
    msg!("Account initialized with zero balances");
    
    Ok(())
}