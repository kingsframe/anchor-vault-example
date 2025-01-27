use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

use crate::state::EscrowState;

#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    pub mint_a: InterfaceAccount<'info, Mint>,
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer=taker,
        associated_token::mint= mint_a,
        associated_token::authority=taker,
     )]
    pub taker_mint_a_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint= mint_b,
        associated_token::authority=taker,
     )]
    pub taker_mint_b_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer=taker,
        associated_token::mint= mint_b,
        associated_token::authority=maker,
     )]
    pub maker_mint_b_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close=maker,
        has_one = maker,
        has_one = mint_a,
        has_one = mint_b,
        seeds=[b"escrow", maker.key.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump=escrow.bump
     )]
    pub escrow: Account<'info, EscrowState>,

    #[account(
        mut,
        associated_token::mint= mint_a,
        associated_token::authority=escrow,
     )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Take<'info> {
    
    pub fn deposit(&mut self) -> Result<()> {
        let transfer_accounts = TransferChecked {
            from: self.taker_mint_b_ata.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_mint_b_ata.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(self.token_program.to_account_info(), transfer_accounts);

        transfer_checked(cpi_ctx, self.escrow.receive_amount, self.mint_b.decimals)
    }

    /// Withdraws the deposited mint_a tokens from the vault to the taker and closes the vault account
    /// This action finalizes the escrow by returning control of the deposited assets to the taker and cleaning up state
    pub fn withdraw_and_close_vault(&mut self) -> Result<()> {
        let signer_seeds: [&[&[u8]]; 1] = [&[
            b"escrow",
            self.maker.to_account_info().key.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ]];

        let accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_mint_a_ata.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            &signer_seeds,
        );
        transfer_checked(ctx, self.vault.amount, self.mint_a.decimals)?;

        let accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.taker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            &signer_seeds,
        );
        close_account(ctx)
    }
}
