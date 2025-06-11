use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_token::instruction as token_instruction;

use crate::state::{MarketState, UserBalance};

pub fn process_settle_balance(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let user_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let market_authority_info = next_account_info(account_info_iter)?;
    let user_base_token_info = next_account_info(account_info_iter)?;
    let user_quote_token_info = next_account_info(account_info_iter)?;
    let market_base_vault_info = next_account_info(account_info_iter)?;
    let market_quote_vault_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    if !user_info.is_signer {
        msg!("User must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let market_state = MarketState::try_from_slice(&market_info.data.borrow())?;
    let mut user_balance = UserBalance::try_from_slice(&user_balance_info.data.borrow())?;

    if user_balance.owner != *user_info.key {
        msg!("User balance account does not belong to signer");
        return Err(ProgramError::InvalidAccountData);
    }

    if user_balance.market != *market_info.key {
        msg!("User balance account does not belong to this market");
        return Err(ProgramError::InvalidAccountData);
    }

    if *market_base_vault_info.key != market_state.base_vault {
        msg!(
            "Market base vault mismatch. Expected: {}, Got: {}",
            market_state.base_vault,
            market_base_vault_info.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    if *market_quote_vault_info.key != market_state.quote_vault {
        msg!(
            "Market quote vault mismatch. Expected: {}, Got: {}",
            market_state.quote_vault,
            market_quote_vault_info.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let settle_base_tokens = user_balance.pending_base_balance > 0;
    let settle_quote_tokens = user_balance.pending_quote_balance > 0;

    if !settle_base_tokens && !settle_quote_tokens {
        msg!("No tokens to settle");
        return Ok(());
    }

    if settle_base_tokens {
        let user_base_token_data = user_base_token_info.try_borrow_data()?;
        if user_base_token_data.len() < 32 {
            msg!("Invalid user base token account data");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&user_base_token_data[0..32]);
        let user_base_token_mint = Pubkey::new_from_array(mint_bytes);

        if user_base_token_mint != market_state.base_mint {
            msg!(
                "User base token account mint mismatch. Expected: {}, Got: {}",
                market_state.base_mint,
                user_base_token_mint
            );
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if settle_quote_tokens {
        let user_quote_token_data = user_quote_token_info.try_borrow_data()?;
        if user_quote_token_data.len() < 32 {
            msg!("Invalid user quote token account data");
            return Err(ProgramError::InvalidAccountData);
        }

        let mut mint_bytes = [0u8; 32];
        mint_bytes.copy_from_slice(&user_quote_token_data[0..32]);
        let user_quote_token_mint = Pubkey::new_from_array(mint_bytes);

        if user_quote_token_mint != market_state.quote_mint {
            msg!(
                "User quote token account mint mismatch. Expected: {}, Got: {}",
                market_state.quote_mint,
                user_quote_token_mint
            );
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let market_seeds = &[
        b"market",
        market_state.base_mint.as_ref(),
        market_state.quote_mint.as_ref(),
        &[market_state.bump],
    ];

    let (expected_market_authority, _) = Pubkey::find_program_address(
        &[
            b"market",
            market_state.base_mint.as_ref(),
            market_state.quote_mint.as_ref(),
        ],
        program_id,
    );

    if *market_authority_info.key != expected_market_authority {
        msg!("Invalid market authority");
        return Err(ProgramError::InvalidAccountData);
    }

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

        user_balance.pending_base_balance = 0;
        msg!("Base tokens settled successfully");
    } else {
        msg!("No base tokens to settle");
    }

    if settle_quote_tokens {
        msg!(
            "Settling {} quote tokens",
            user_balance.pending_quote_balance
        );

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

        user_balance.pending_quote_balance = 0;
        msg!("Quote tokens settled successfully");
    } else {
        msg!("No quote tokens to settle");
    }

    user_balance.serialize(&mut *user_balance_info.data.borrow_mut())?;

    msg!("Settlement completed successfully");
    Ok(())
}
