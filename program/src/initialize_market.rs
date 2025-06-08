use borsh::BorshSerialize;
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

use crate::state::{MarketEvents, MarketState, MAX_EVENTS};

pub fn process_initialize_market(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    min_order_size: u64,
    tick_size: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let authority_info = next_account_info(account_info_iter)?;
    let market_info = next_account_info(account_info_iter)?;
    let base_mint_info = next_account_info(account_info_iter)?;
    let quote_mint_info = next_account_info(account_info_iter)?;
    let base_vault_info = next_account_info(account_info_iter)?;
    let quote_vault_info = next_account_info(account_info_iter)?;
    let market_events_info = next_account_info(account_info_iter)?;
    let fee_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    if !(authority_info.is_signer && authority_info.is_writable) {
        msg!("Authority must be a signer and writable");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let accounts_to_validate = [
        (market_info, "Market"),
        (market_events_info, "Market events"),
        (base_vault_info, "Base vault"),
        (quote_vault_info, "Quote vault"),
        (fee_account_info, "Fee account"),
    ];

    for (account, name) in accounts_to_validate.iter() {
        if !account.is_writable {
            msg!("{} account must be writable", name);
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if !system_program::check_id(system_program_info.key) {
        msg!("Invalid system program");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_token::check_id(token_program_info.key) {
        msg!("Invalid token program");
        return Err(ProgramError::IncorrectProgramId);
    }

    if min_order_size == 0 {
        msg!("Minimum order size must be greater than 0");
        return Err(ProgramError::InvalidInstructionData);
    }

    if tick_size == 0 {
        msg!("Tick size must be greater than 0");
        return Err(ProgramError::InvalidInstructionData);
    }

    let market_seeds = &[
        b"market",
        base_mint_info.key.as_ref(),
        quote_mint_info.key.as_ref(),
    ];
    let (market_pda, bump) = Pubkey::find_program_address(market_seeds, program_id);

    if market_info.key != &market_pda {
        msg!("Invalid market account. Expected PDA: {}", market_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    let market_events_seeds = &[b"events", market_pda.as_ref()];
    let (market_events_pda, events_bump) =
        Pubkey::find_program_address(market_events_seeds, program_id);

    if market_events_info.key != &market_events_pda {
        msg!(
            "Invalid market events account. Expected PDA: {}",
            market_events_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let base_vault_seeds = &[b"base_vault", market_pda.as_ref()];
    let (base_vault_pda, base_vault_bump) =
        Pubkey::find_program_address(base_vault_seeds, program_id);

    if base_vault_info.key != &base_vault_pda {
        msg!(
            "Invalid base vault account. Expected PDA: {}",
            base_vault_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let quote_vault_seeds = &[b"quote_vault", market_pda.as_ref()];
    let (quote_vault_pda, quote_vault_bump) =
        Pubkey::find_program_address(quote_vault_seeds, program_id);

    if quote_vault_info.key != &quote_vault_pda {
        msg!(
            "Invalid quote vault account. Expected PDA: {}",
            quote_vault_pda
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let fee_account_seeds = &[b"fee_account", market_pda.as_ref()];
    let (fee_account_pda, fee_account_bump) =
        Pubkey::find_program_address(fee_account_seeds, program_id);

    if fee_account_info.key != &fee_account_pda {
        msg!("Invalid fee account. Expected PDA: {}", fee_account_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    let rent = Rent::from_account_info(rent_info)?;

    if market_info.lamports() == 0 {
        msg!("Creating market account with {} bytes", MarketState::LEN);
        let market_rent = rent.minimum_balance(MarketState::LEN);
        let create_market_ix = system_instruction::create_account(
            authority_info.key,
            &market_pda,
            market_rent,
            MarketState::LEN as u64,
            program_id,
        );

        invoke_signed(
            &create_market_ix,
            &[
                authority_info.clone(),
                market_info.clone(),
                system_program_info.clone(),
            ],
            &[&[
                b"market",
                base_mint_info.key.as_ref(),
                quote_mint_info.key.as_ref(),
                &[bump],
            ]],
        )?;
    }

    if market_events_info.lamports() == 0 {
        let events_rent = rent.minimum_balance(MarketEvents::MIN_LEN);
        let create_events_ix = system_instruction::create_account(
            authority_info.key,
            &market_events_pda,
            events_rent,
            MarketEvents::MIN_LEN as u64,
            program_id,
        );

        invoke_signed(
            &create_events_ix,
            &[
                authority_info.clone(),
                market_events_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"events", market_pda.as_ref(), &[events_bump]]],
        )?;
    }

    if base_vault_info.lamports() == 0 {
        let vault_rent = rent.minimum_balance(165);
        let create_base_vault_ix = system_instruction::create_account(
            authority_info.key,
            &base_vault_pda,
            vault_rent,
            165,
            &spl_token::id(),
        );

        invoke_signed(
            &create_base_vault_ix,
            &[
                authority_info.clone(),
                base_vault_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"base_vault", market_pda.as_ref(), &[base_vault_bump]]],
        )?;
    }

    if quote_vault_info.lamports() == 0 {
        let vault_rent = rent.minimum_balance(165);
        let create_quote_vault_ix = system_instruction::create_account(
            authority_info.key,
            &quote_vault_pda,
            vault_rent,
            165,
            &spl_token::id(),
        );

        invoke_signed(
            &create_quote_vault_ix,
            &[
                authority_info.clone(),
                quote_vault_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"quote_vault", market_pda.as_ref(), &[quote_vault_bump]]],
        )?;
    }

    if fee_account_info.lamports() == 0 {
        let fee_rent = rent.minimum_balance(165);
        let create_fee_account_ix = system_instruction::create_account(
            authority_info.key,
            &fee_account_pda,
            fee_rent,
            165,
            &spl_token::id(),
        );

        invoke_signed(
            &create_fee_account_ix,
            &[
                authority_info.clone(),
                fee_account_info.clone(),
                system_program_info.clone(),
            ],
            &[&[b"fee_account", market_pda.as_ref(), &[fee_account_bump]]],
        )?;
    }

    if base_mint_info.lamports() == 0 {
        msg!("Base mint account does not exist: {}", base_mint_info.key);
        return Err(ProgramError::InvalidAccountData);
    }
    
    if quote_mint_info.lamports() == 0 {
        msg!("Quote mint account does not exist: {}", quote_mint_info.key);
        return Err(ProgramError::InvalidAccountData);
    }

    let init_base_vault_ix = token_instruction::initialize_account(
        token_program_info.key,
        base_vault_info.key,
        base_mint_info.key,
        &market_pda,
    )?;

    invoke(
        &init_base_vault_ix,
        &[
            base_vault_info.clone(),
            base_mint_info.clone(),
            market_info.clone(), 
            token_program_info.clone(),
            rent_info.clone(),
        ],
    )?;

    let init_quote_vault_ix = token_instruction::initialize_account(
        token_program_info.key,
        quote_vault_info.key,
        quote_mint_info.key,
        &market_pda,
    )?;

    invoke(
        &init_quote_vault_ix,
        &[
            quote_vault_info.clone(),
            quote_mint_info.clone(),
            market_info.clone(),  
            token_program_info.clone(),
            rent_info.clone(),
        ],
    )?;

    let init_fee_account_ix = token_instruction::initialize_account(
        token_program_info.key,
        fee_account_info.key,
        quote_mint_info.key,
        &market_pda,
    )?;

    invoke(
        &init_fee_account_ix,
        &[
            fee_account_info.clone(),
            quote_mint_info.clone(),
            market_info.clone(),  
            token_program_info.clone(),
            rent_info.clone(),
        ],
    )?;

    let market_state = MarketState {
        authority: *authority_info.key,
        base_mint: *base_mint_info.key,
        quote_mint: *quote_mint_info.key,
        fee_account: *fee_account_info.key,
        base_vault: *base_vault_info.key,
        quote_vault: *quote_vault_info.key,
        market_events: *market_events_info.key,
        event_head: 0,
        event_tail: 0,
        min_order_size,
        tick_size,
        next_order_id: 1,
        total_events: 0,
        last_price: 0,
        volume_24h: 0,
        fee_rate_bps: 30,
        bump: bump,
        is_initialized: true,
    };
    msg!("MarketState struct created successfully");

    let mut data = market_info.data.borrow_mut();
    let mut serialized_data = Vec::new();
    market_state.serialize(&mut serialized_data)?;
    
    data[..serialized_data.len()].copy_from_slice(&serialized_data);
    msg!("MarketState serialized successfully");

    let market_events = MarketEvents::new(market_pda);
    let mut events_data = market_events_info.data.borrow_mut();
    let mut serialized_events = Vec::new();
    market_events.serialize(&mut serialized_events)?;
    events_data[..serialized_events.len()].copy_from_slice(&serialized_events);
    msg!("MarketEvents serialized successfully");

    msg!("Market PDA: {}", market_pda);
    msg!("Events PDA: {}", market_events_pda);
    msg!("Authority: {}", authority_info.key);
    msg!("Base mint: {}", base_mint_info.key);
    msg!("Quote mint: {}", quote_mint_info.key);
    msg!("Min order size: {}", min_order_size);
    msg!("Tick size: {}", tick_size);
    msg!("Fee account: {}", fee_account_pda);
    msg!("Base vault: {}", base_vault_pda);
    msg!("Quote vault: {}", quote_vault_pda);
    msg!("Max events: {}", MAX_EVENTS);

    Ok(())
} 