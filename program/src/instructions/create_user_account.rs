use crate::state::{MarketState, UserBalance};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};
use spl_token::instruction as token_instruction;

pub fn process_create_acc_and_deposit_quote_tokens(
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

    let new_user_account_seeds = &[
        b"user_balance",
        user_info.key.as_ref(),
        market_info.key.as_ref(),
    ];
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
    let account_exists = new_user_balance_info.lamports() > 0
        && new_user_balance_info.data_len() == UserBalance::LEN;

    if !account_exists {
        msg!(
            "Creating user balance account with {} bytes",
            UserBalance::LEN
        );
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
            &[&[
                b"user_balance",
                user_info.key.as_ref(),
                market_info.key.as_ref(),
                &[new_user_account_bump],
            ]],
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

        let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;

        if *quote_vault_info.key != market_state.quote_vault {
            msg!(
                "Quote vault mismatch. Expected: {}, Got: {}",
                market_state.quote_vault,
                quote_vault_info.key
            );
            return Err(ProgramError::InvalidAccountData);
        }

        //first 32 bytes of spl token account consist of mint,for ref check state on spl_token mod
        let user_token_data = user_quote_token_account_info.try_borrow_data()?;
        if user_token_data.len() < 32 {
            msg!("Invalid user quote token account data");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&user_token_data[0..32]);
        let user_token_mint = Pubkey::new_from_array(mint_bytes);

        if user_token_mint != market_state.quote_mint {
            msg!(
                "User quote token account mint mismatch. Expected: {}, Got: {}",
                market_state.quote_mint,
                user_token_mint
            );
            return Err(ProgramError::InvalidAccountData);
        }

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

        msg!(
            "Successfully deposited {} tokens to user balance",
            onramp_quantity
        );
        msg!(
            "New available quote balance: {}",
            user_balance.available_quote_balance
        );
    }

    Ok(())
}

pub fn process_create_acc_and_deposit_base_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    quantity: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let user_base_token_account_info = next_account_info(account_info_iter)?;
    let base_vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if !user_info.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !spl_token::check_id(token_program_info.key) {
        msg!("Invalid token program");
        return Err(ProgramError::IncorrectProgramId);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;

    if *base_vault_info.key != market_state.base_vault {
        msg!(
            "Base vault mismatch. Expected: {}, Got: {}",
            market_state.base_vault,
            base_vault_info.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    //first 32 bytes of spl token account consist of mint,for ref check state on spl_token mod
    let user_token_data = user_base_token_account_info.try_borrow_data()?;
    if user_token_data.len() < 32 {
        msg!("Invalid user base token account data");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut mint_bytes = [0u8; 32];
    mint_bytes.copy_from_slice(&user_token_data[0..32]);
    let user_token_mint = Pubkey::new_from_array(mint_bytes);

    if user_token_mint != market_state.base_mint {
        msg!(
            "User base token account mint mismatch. Expected: {}, Got: {}",
            market_state.base_mint,
            user_token_mint
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let user_balance_seeds = &[
        b"user_balance",
        user_info.key.as_ref(),
        market_info.key.as_ref(),
    ];
    let (expected_user_balance_pda, _) =
        Pubkey::find_program_address(user_balance_seeds, program_id);

    if user_balance_info.key != &expected_user_balance_pda {
        msg!(
            "Invalid user balance account. Expected PDA: {}",
            expected_user_balance_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    if quantity == 0 {
        msg!("Quantity must be greater than 0");
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;

    if user_balance.owner != *user_info.key {
        msg!("User balance account does not belong to signer");
        return Err(ProgramError::InvalidAccountData);
    }

    if user_balance.market != *market_info.key {
        msg!("User balance account does not belong to this market");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("Depositing {} base tokens", quantity);

    let transfer_ix = token_instruction::transfer(
        token_program_info.key,
        user_base_token_account_info.key,
        base_vault_info.key,
        user_info.key,
        &[],
        quantity,
    )?;

    invoke(
        &transfer_ix,
        &[
            user_base_token_account_info.clone(),
            base_vault_info.clone(),
            user_info.clone(),
            token_program_info.clone(),
        ],
    )?;

    user_balance.available_base_balance += quantity;
    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;

    msg!(
        "Successfully deposited {} base tokens to user balance",
        quantity
    );
    msg!(
        "New available base balance: {}",
        user_balance.available_base_balance
    );

    Ok(())
}
