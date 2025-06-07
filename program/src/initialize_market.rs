use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct MarketState {
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub min_order_size: u64,
    pub tick_size: u64,
    pub next_order_id: u64,
    pub is_active: bool,
}

impl MarketState {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 8 + 1;
}

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
    let system_program_info = next_account_info(account_info_iter)?;

    if !authority_info.is_signer {
        msg!("Authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if !market_info.is_writable {
        msg!("Market account must be writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if !system_program::check_id(system_program_info.key) {
        msg!("Invalid system program");
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
    let (market_pda, bump_seed) = Pubkey::find_program_address(market_seeds, program_id);

    if market_info.key != &market_pda {
        msg!("Invalid market account. Expected PDA: {}", market_pda);
        return Err(ProgramError::InvalidAccountData);
    }

    if market_info.data_len() > 0 && !market_info.data.borrow().iter().all(|&x| x == 0) {
        msg!("Market account is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(MarketState::LEN);

    if market_info.data_len() == 0 {
        msg!("Creating market PDA with {} lamports", required_lamports);

        let market_signer_seeds = &[
            b"market",
            base_mint_info.key.as_ref(),
            quote_mint_info.key.as_ref(),
            &[bump_seed],
        ];

        invoke_signed(
            &system_instruction::create_account(
                authority_info.key,
                market_info.key,
                required_lamports,
                MarketState::LEN as u64,
                program_id,
            ),
            &[
                authority_info.clone(),
                market_info.clone(),
                system_program_info.clone(),
            ],
            &[market_signer_seeds],
        )?;
    }

    let market_state = MarketState {
        authority: *authority_info.key,
        base_mint: *base_mint_info.key,
        quote_mint: *quote_mint_info.key,
        min_order_size,
        tick_size,
        next_order_id: 1,
        is_active: true,
    };

    market_state.serialize(&mut &mut market_info.data.borrow_mut()[..])?;

    msg!("Market initialized successfully!");
    msg!("Market PDA: {}", market_pda);
    msg!("Authority: {}", authority_info.key);
    msg!("Base mint: {}", base_mint_info.key);
    msg!("Quote mint: {}", quote_mint_info.key);
    msg!("Min order size: {}", min_order_size);
    msg!("Tick size: {}", tick_size);

    Ok(())
}
