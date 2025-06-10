use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program::{invoke_signed},
    pubkey::Pubkey,
};
use spl_token::instruction as token_instruction;

use crate::state::{MarketState, UserBalance};

pub fn process_settle_balance(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_authority_info = next_account_info(account_info_iter)?;
    let base_mint_info = next_account_info(account_info_iter)?;
    let quote_mint_info = next_account_info(account_info_iter)?;
    let user_base_token_info = next_account_info(account_info_iter)?;
    let user_quote_token_info = next_account_info(account_info_iter)?;
    let market_base_vault_info = next_account_info(account_info_iter)?;
    let market_quote_vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    // Verify user is signer
    if !user_info.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;

    // Verify user balance account belongs to the signer
    if user_balance.owner != *user_info.key {
        msg!("User balance account does not belong to signer");
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify market relationship
    if user_balance.market != *market_info.key {
        msg!("User balance account does not belong to this market");
        return Err(ProgramError::InvalidAccountData);
    }

    // Check if there are base tokens to settle
    let settle_base_tokens = user_balance.pending_base_balance > 0;
    let settle_quote_tokens = user_balance.pending_quote_balance > 0;

    if !settle_base_tokens && !settle_quote_tokens {
        msg!("No tokens to settle");
        return Ok(());
    }

    // Derive market authority PDA
    let market_seeds = &[
        b"market",
        market_state.base_mint.as_ref(),
        market_state.quote_mint.as_ref(),
        &[market_state.bump],
    ];

    // Verify market authority PDA
    let (expected_market_authority, _) = Pubkey::find_program_address(
        &[b"market", market_state.base_mint.as_ref(), market_state.quote_mint.as_ref()],
        _program_id,
    );
    
    if *market_authority_info.key != expected_market_authority {
        msg!("Invalid market authority");
        return Err(ProgramError::InvalidAccountData);
    }

    // Settle base tokens if any
    if settle_base_tokens {
        msg!("Settling {} base tokens", user_balance.pending_base_balance);

        let transfer_base_ix = token_instruction::transfer(
            token_program_info.key,
            market_base_vault_info.key,
            user_base_token_info.key,
            market_authority_info.key,
            &[],
            user_balance.pending_base_balance,
        )?;

        invoke_signed(
            &transfer_base_ix,
            &[
                market_base_vault_info.clone(),
                user_base_token_info.clone(),
                market_authority_info.clone(),
                token_program_info.clone(),
            ],
            &[market_seeds],
        )?;

        // Reset pending base balance
        user_balance.pending_base_balance = 0;
        msg!("Base tokens settled successfully");
    } else {
        msg!("No base tokens to settle");
    }

    // Settle quote tokens if any
    if settle_quote_tokens {
        msg!("Settling {} quote tokens", user_balance.pending_quote_balance);

        let transfer_quote_ix = token_instruction::transfer(
            token_program_info.key,
            market_quote_vault_info.key,
            user_quote_token_info.key,
            market_authority_info.key,
            &[],
            user_balance.pending_quote_balance,
        )?;

        invoke_signed(
            &transfer_quote_ix,
            &[
                market_quote_vault_info.clone(),
                user_quote_token_info.clone(),
                market_authority_info.clone(),
                token_program_info.clone(),
            ],
            &[market_seeds],
        )?;

        // Reset pending quote balance
        user_balance.pending_quote_balance = 0;
        msg!("Quote tokens settled successfully");
    } else {
        msg!("No quote tokens to settle");
    }

    // Serialize updated user balance
    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;

    msg!("Settlement completed successfully");
    Ok(())
} 