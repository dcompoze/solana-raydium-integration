use std::{rc::Rc, str::FromStr};

use anchor_client::{
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        program_pack::Pack,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signature, Signer},
        system_program, sysvar,
        transaction::Transaction,
    },
    Client, Cluster, Program,
};
use anyhow::{anyhow, Context, Result};
use raydium_cp_swap::{
    accounts,
    curve::{CurveCalculator, RoundDirection},
    instruction,
    states::{
        pool::{POOL_LP_MINT_SEED, POOL_SEED, POOL_VAULT_SEED},
        AmmConfig, PoolState, AMM_CONFIG_SEED, OBSERVATION_SEED,
    },
    AUTH_SEED,
};
use solana_program::instruction::Instruction;
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
    instruction::create_associated_token_account_idempotent,
};
use spl_token::state::Account;

const RPC_URL: &str = "https://api.devnet.solana.com";
const WALLET_FILE: &str = "./devnet.json";
const TOKEN_A: &str = "69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho";
const TOKEN_B: &str = "2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU";

#[derive(Debug)]
pub struct InitializationKeys {
    /// Pool's vault account for token 0.
    pub token_0_vault: Pubkey,
    /// Pool's vault account for token 1.
    pub token_1_vault: Pubkey,
    /// Pool's state account.
    pub pool_state: Pubkey,
    /// Pool's authority account.
    pub pool_authority: Pubkey,
    /// Mint account for the pool's LP tokens.
    pub lp_mint: Pubkey,
    /// Creator ATA for token 0.
    pub creator_token_0: Pubkey,
    /// Creator ATA for token 1.
    pub creator_token_1: Pubkey,
    /// Creator ATA for LP tokens.
    pub creator_lp_ata: Pubkey,
}

#[derive(Debug)]
pub struct PoolLiquidity {
    /// Amount of token 0 in the pool.
    pub token_0_amount: u64,
    /// Amount of token 1 in the pool.
    pub token_1_amount: u64,
    /// Total supply of LP tokens.
    pub lp_supply: u64,
}

struct RaydiumIntegration {
    client_rpc: RpcClient,
    program: Program<Rc<Keypair>>,
    payer: Rc<Keypair>,
}

impl RaydiumIntegration {
    /// Creates a new Raydium integration with initialized clients and payer wallet.
    pub fn new(payer: Rc<Keypair>, cluster: Cluster) -> Result<Self> {
        let client_rpc = RpcClient::new(RPC_URL);
        let client_anchor =
            Client::new_with_options(cluster, payer.clone(), CommitmentConfig::confirmed());
        let program = client_anchor.program(raydium_cp_swap::id())?;

        Ok(Self {
            client_rpc,
            program,
            payer,
        })
    }

    /// Initializes a new Raydium CP-AMM pool or returns data from an existing pool.
    pub fn initialize_pool(
        &self,
        amm_config_key: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_amount: u64,
        token_1_amount: u64,
        open_time: u64,
    ) -> Result<(Option<Signature>, InitializationKeys)> {
        if token_0_amount == 0 || token_1_amount == 0 {
            return Err(anyhow!("initial amounts cannot be zero"));
        }

        // Get the pool accounts and check if the pool already exists.
        // If it exists return the data from the pool state account instead of initializing the pool.
        let token_0_program = self
            .client_rpc
            .get_account(&token_0_mint)
            .context("failed to get token_0_mint owner")?
            .owner;

        let token_1_program = self
            .client_rpc
            .get_account(&token_1_mint)
            .context("failed to get token_1_mint owner")?
            .owner;

        let (pool_state, _bump) = Pubkey::find_program_address(
            &[
                POOL_SEED.as_bytes(),
                amm_config_key.to_bytes().as_ref(),
                token_0_mint.to_bytes().as_ref(),
                token_1_mint.to_bytes().as_ref(),
            ],
            &self.program.id(),
        );

        let (pool_authority, _bump) =
            Pubkey::find_program_address(&[AUTH_SEED.as_bytes()], &self.program.id());

        let creator_token_0 = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &token_0_mint,
            &token_0_program,
        );

        let creator_token_1 = get_associated_token_address_with_program_id(
            &self.payer.pubkey(),
            &token_1_mint,
            &token_1_program,
        );

        if let Ok(pool_data) = self.program.account::<PoolState>(pool_state) {
            log::info!(
                "Pool already exists for tokens {} and {}",
                token_0_mint,
                token_1_mint
            );

            let token_0_vault = pool_data.token_0_vault;
            let token_1_vault = pool_data.token_1_vault;
            let lp_mint = pool_data.lp_mint;
            let creator_lp_ata = get_associated_token_address(&self.payer.pubkey(), &lp_mint);

            return Ok((
                None,
                InitializationKeys {
                    token_0_vault,
                    token_1_vault,
                    pool_state,
                    pool_authority,
                    lp_mint,
                    creator_token_0,
                    creator_token_1,
                    creator_lp_ata,
                },
            ));
        }

        log::info!(
            "Initializing pool with tokens {} ({}) and {} ({})",
            token_0_mint,
            token_0_amount,
            token_1_mint,
            token_1_amount
        );

        // Get other accounts related to the program.
        let (token_0_vault, _bump) = Pubkey::find_program_address(
            &[
                POOL_VAULT_SEED.as_bytes(),
                pool_state.to_bytes().as_ref(),
                token_0_mint.to_bytes().as_ref(),
            ],
            &self.program.id(),
        );

        let (token_1_vault, _bump) = Pubkey::find_program_address(
            &[
                POOL_VAULT_SEED.as_bytes(),
                pool_state.to_bytes().as_ref(),
                token_1_mint.to_bytes().as_ref(),
            ],
            &self.program.id(),
        );

        let (lp_mint, _bump) = Pubkey::find_program_address(
            &[POOL_LP_MINT_SEED.as_bytes(), pool_state.to_bytes().as_ref()],
            &self.program.id(),
        );

        let (observation_state, _bump) = Pubkey::find_program_address(
            &[OBSERVATION_SEED.as_bytes(), pool_state.to_bytes().as_ref()],
            &self.program.id(),
        );

        let creator_lp_ata = get_associated_token_address(&self.payer.pubkey(), &lp_mint);

        let initialization_accounts = accounts::Initialize {
            creator: self.payer.pubkey(),
            amm_config: amm_config_key,
            authority: pool_authority,
            pool_state,
            token_0_mint,
            token_1_mint,
            lp_mint,
            creator_token_0,
            creator_token_1,
            creator_lp_token: creator_lp_ata,
            token_0_vault,
            token_1_vault,
            create_pool_fee: raydium_cp_swap::create_pool_fee_reveiver::id(),
            observation_state,
            token_program: spl_token::id(),
            token_0_program,
            token_1_program,
            associated_token_program: spl_associated_token_account::id(),
            system_program: system_program::id(),
            rent: sysvar::rent::id(),
        };

        let initialization_args = instruction::Initialize {
            init_amount_0: token_0_amount,
            init_amount_1: token_1_amount,
            open_time,
        };

        let initialization_instructions = self
            .program
            .request()
            .accounts(initialization_accounts)
            .args(initialization_args)
            .instructions()
            .context("failed to build initialization instructions")?;

        let recent_blockhash = self
            .client_rpc
            .get_latest_blockhash()
            .context("failed to get recent blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &initialization_instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self
            .client_rpc
            .send_and_confirm_transaction_with_spinner(&transaction)
            .context("failed to send initialization transaction")?;

        Ok((
            Some(signature),
            InitializationKeys {
                token_0_vault,
                token_1_vault,
                pool_state,
                pool_authority,
                lp_mint,
                creator_token_0,
                creator_token_1,
                creator_lp_ata,
            },
        ))
    }

    /// Adds liquidity to a Raydium CP-AMM pool.
    pub fn add_liquidity(
        &self,
        pool_state: Pubkey,
        pool_authority: Pubkey,
        lp_mint: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        owner_token_0: Pubkey,
        owner_token_1: Pubkey,
        owner_lp: Pubkey,
        lp_token_amount: u64,
    ) -> Result<Signature> {
        let tx_instructions = self.create_deposit_instructions(
            pool_state,
            pool_authority,
            lp_mint,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            owner_token_0,
            owner_token_1,
            owner_lp,
            lp_token_amount,
        )?;

        let recent_blockhash = self
            .client_rpc
            .get_latest_blockhash()
            .context("failed to get recent blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &tx_instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self
            .client_rpc
            .send_and_confirm_transaction_with_spinner(&transaction)
            .context("failed to send add_liquidity transaction")?;

        Ok(signature)
    }

    /// Creates instructions for depositing liquidity to a Raydium CP-AMM pool.
    fn create_deposit_instructions(
        &self,
        pool_state: Pubkey,
        pool_authority: Pubkey,
        lp_mint: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        owner_token_0: Pubkey,
        owner_token_1: Pubkey,
        owner_lp: Pubkey,
        lp_token_amount: u64,
    ) -> Result<Vec<Instruction>> {
        let mut tx_instructions = Vec::new();

        // Create LP token ATA if it doesn't exist.
        let create_ata_instructions = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &lp_mint,
            &spl_token::id(),
        );
        tx_instructions.push(create_ata_instructions);

        let deposit_accounts = accounts::Deposit {
            owner: self.payer.pubkey(),
            authority: pool_authority,
            pool_state,
            owner_lp_token: owner_lp,
            token_0_account: owner_token_0,
            token_1_account: owner_token_1,
            token_0_vault,
            token_1_vault,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
            vault_0_mint: token_0_mint,
            vault_1_mint: token_1_mint,
            lp_mint,
        };

        let (max_token_0, max_token_1) =
            self.calculate_token_amounts(pool_state, lp_token_amount, true)?;

        let deposit_args = instruction::Deposit {
            lp_token_amount,
            maximum_token_0_amount: max_token_0,
            maximum_token_1_amount: max_token_1,
        };

        let deposit_instructions = self
            .program
            .request()
            .accounts(deposit_accounts)
            .args(deposit_args)
            .instructions()
            .context("failed to build deposit instructions")?;

        tx_instructions.extend(deposit_instructions);
        Ok(tx_instructions)
    }

    /// Removes liquidity from a Raydium CP-AMM pool.
    pub fn remove_liquidity(
        &self,
        pool_state: Pubkey,
        pool_authority: Pubkey,
        lp_mint: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        owner_token_0: Pubkey,
        owner_token_1: Pubkey,
        owner_lp: Pubkey,
        lp_token_amount: u64,
    ) -> Result<Signature> {
        let tx_instructions = self.create_withdrawal_instructions(
            pool_state,
            pool_authority,
            lp_mint,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            owner_token_0,
            owner_token_1,
            owner_lp,
            lp_token_amount,
        )?;

        let recent_blockhash = self
            .client_rpc
            .get_latest_blockhash()
            .context("failed to get recent blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &tx_instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self
            .client_rpc
            .send_and_confirm_transaction_with_spinner(&transaction)
            .context("failed to send remove_liquidity transaction")?;

        Ok(signature)
    }

    /// Creates instructions for withdrawing liquidity from a Raydium CP-AMM pool.
    fn create_withdrawal_instructions(
        &self,
        pool_state: Pubkey,
        pool_authority: Pubkey,
        lp_mint: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        owner_token_0: Pubkey,
        owner_token_1: Pubkey,
        owner_lp: Pubkey,
        lp_token_amount: u64,
    ) -> Result<Vec<Instruction>> {
        let mut tx_instructions = Vec::new();

        // Create token ATAs if they don't exist.
        let create_token_0_ata = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &token_0_mint,
            &spl_token::id(),
        );
        tx_instructions.push(create_token_0_ata);

        let create_token_1_ata = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &self.payer.pubkey(),
            &token_1_mint,
            &spl_token::id(),
        );
        tx_instructions.push(create_token_1_ata);

        let withdrawal_accounts = accounts::Withdraw {
            owner: self.payer.pubkey(),
            authority: pool_authority,
            pool_state,
            owner_lp_token: owner_lp,
            token_0_account: owner_token_0,
            token_1_account: owner_token_1,
            token_0_vault,
            token_1_vault,
            token_program: spl_token::id(),
            token_program_2022: spl_token_2022::id(),
            vault_0_mint: token_0_mint,
            vault_1_mint: token_1_mint,
            lp_mint,
            memo_program: spl_memo::id(),
        };

        let (min_token_0, min_token_1) =
            self.calculate_token_amounts(pool_state, lp_token_amount, false)?;

        let withdrawal_args = instruction::Withdraw {
            lp_token_amount,
            minimum_token_0_amount: min_token_0,
            minimum_token_1_amount: min_token_1,
        };

        let withdrawal_instructions = self
            .program
            .request()
            .accounts(withdrawal_accounts)
            .args(withdrawal_args)
            .instructions()
            .context("failed to build withdraw instructions")?;

        tx_instructions.extend(withdrawal_instructions);
        Ok(tx_instructions)
    }

    /// Adds and removes liquidity from a Raydium CP-AMM pool in a single transaction.
    pub fn add_and_remove_liquidity(
        &self,
        pool_state: Pubkey,
        pool_authority: Pubkey,
        lp_mint: Pubkey,
        token_0_mint: Pubkey,
        token_1_mint: Pubkey,
        token_0_vault: Pubkey,
        token_1_vault: Pubkey,
        owner_token_0: Pubkey,
        owner_token_1: Pubkey,
        owner_lp: Pubkey,
        lp_token_amount: u64,
    ) -> Result<Signature> {
        let mut tx_instructions = self.create_deposit_instructions(
            pool_state,
            pool_authority,
            lp_mint,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            owner_token_0,
            owner_token_1,
            owner_lp,
            lp_token_amount,
        )?;

        tx_instructions.extend(self.create_withdrawal_instructions(
            pool_state,
            pool_authority,
            lp_mint,
            token_0_mint,
            token_1_mint,
            token_0_vault,
            token_1_vault,
            owner_token_0,
            owner_token_1,
            owner_lp,
            lp_token_amount,
        )?);

        let recent_blockhash = self
            .client_rpc
            .get_latest_blockhash()
            .context("failed to get recent blockhash")?;

        let transaction = Transaction::new_signed_with_payer(
            &tx_instructions,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            recent_blockhash,
        );

        let signature = self
            .client_rpc
            .send_and_confirm_transaction_with_spinner(&transaction)
            .context("failed to send add_and_remove_liquidity transaction")?;

        Ok(signature)
    }

    /// Dynamically calculate token amounts needed for deposit or expected from withdrawal.
    fn calculate_token_amounts(
        &self,
        pool_state: Pubkey,
        lp_token_amount: u64,
        deposit: bool,
    ) -> Result<(u64, u64)> {
        let pool_liquidity = self.get_pool_liquidity(pool_state)?;

        // Calculate base amounts using Raydium's CurveCalculator.
        let results = CurveCalculator::lp_tokens_to_trading_tokens(
            u128::from(lp_token_amount),
            u128::from(pool_liquidity.lp_supply),
            u128::from(pool_liquidity.token_0_amount),
            u128::from(pool_liquidity.token_1_amount),
            RoundDirection::Ceiling,
        )
        .ok_or(anyhow!("failed to calculate amounts"))?;

        if results.token_0_amount > u64::MAX as u128 || results.token_1_amount > u64::MAX as u128 {
            return Err(anyhow!("token amount too large for u64"));
        }
        let token_0_amount = results.token_0_amount as u64;
        let token_1_amount = results.token_1_amount as u64;

        const SLIPPAGE: f64 = 0.01; // Hardcoded for simplicity.

        if deposit {
            // For deposits, add slippage to get maximum amounts.
            let max_amount_0 = (token_0_amount as f64 * (1.0 + SLIPPAGE)).ceil() as u64;
            let max_amount_1 = (token_1_amount as f64 * (1.0 + SLIPPAGE)).ceil() as u64;
            Ok((max_amount_0, max_amount_1))
        } else {
            // For withdrawals, subtract slippage to get minimum amounts.
            let min_amount_0 = (token_0_amount as f64 * (1.0 - SLIPPAGE)).floor() as u64;
            let min_amount_1 = (token_1_amount as f64 * (1.0 - SLIPPAGE)).floor() as u64;
            Ok((min_amount_0, min_amount_1))
        }
        // NOTE: Token-2022 transfer fee calculations are skipped for simplicity.
    }

    /// Lists available AMM configurations.
    pub fn list_amm_configs(&self) -> Result<()> {
        let configs: Vec<(Pubkey, AmmConfig)> =
            self.program.accounts(vec![])?.into_iter().collect();

        for (address, config) in configs {
            log::info!(
                "AMM config {}: index={}, trade_fee_rate={}, protocol_fee_rate={}",
                address,
                config.index,
                config.trade_fee_rate,
                config.protocol_fee_rate
            );
        }
        Ok(())
    }

    /// Returns an AMM configuration for the specified index if it exists.
    pub fn get_amm_config_by_index(&self, index: u16) -> Result<(Pubkey, AmmConfig)> {
        let (amm_config_key, _) = Pubkey::find_program_address(
            &[AMM_CONFIG_SEED.as_bytes(), &index.to_be_bytes()],
            &self.program.id(),
        );

        let config = self.program.account::<AmmConfig>(amm_config_key)?;
        Ok((amm_config_key, config))
    }

    /// Fetches the current liquidity amounts from a Raydium CP-AMM pool.
    pub fn get_pool_liquidity(&self, pool_state: Pubkey) -> Result<PoolLiquidity> {
        let pool_data = self
            .program
            .account::<PoolState>(pool_state)
            .context("failed to fetch pool state")?;

        let vault_accounts = self
            .client_rpc
            .get_multiple_accounts(&[pool_data.token_0_vault, pool_data.token_1_vault])?;

        let [token_0_vault_account, token_1_vault_account] = match vault_accounts.as_slice() {
            [Some(a), Some(b)] => [a, b],
            _ => return Err(anyhow!("failed to fetch vault accounts")),
        };

        let token_0_vault_info = Account::unpack(&token_0_vault_account.data)?;
        let token_1_vault_info = Account::unpack(&token_1_vault_account.data)?;

        let (total_token_0_amount, total_token_1_amount) = pool_data
            .vault_amount_without_fee(token_0_vault_info.amount, token_1_vault_info.amount);

        Ok(PoolLiquidity {
            token_0_amount: total_token_0_amount,
            token_1_amount: total_token_1_amount,
            lp_supply: pool_data.lp_supply,
        })
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let payer = Rc::new(
        read_keypair_file(WALLET_FILE)
            .map_err(|e| anyhow!("failed to read keypair file: {}", e))?,
    );
    let raydium = RaydiumIntegration::new(payer.clone(), Cluster::Devnet)?;

    raydium.list_amm_configs()?;
    const AMM_CONFIG_INDEX: u16 = 0;
    log::info!("Using AMM config with index: {AMM_CONFIG_INDEX}");
    let (amm_config_key, _) = raydium.get_amm_config_by_index(AMM_CONFIG_INDEX)?;

    // STEP 1: Initialize the CP-AMM pool with 1000 of each token (6 decimals).

    let token_a = Pubkey::from_str(TOKEN_A)?;
    let token_b = Pubkey::from_str(TOKEN_B)?;
    let (token_0_mint, token_1_mint) = order_tokens(token_a, token_b);
    const TOKEN_0_AMOUNT: u64 = 1_000_000_000;
    const TOKEN_1_AMOUNT: u64 = 1_000_000_000;
    const OPEN_TIME: u64 = 0; // Immediatelly

    let (signature, init_keys) = raydium.initialize_pool(
        amm_config_key,
        token_0_mint,
        token_1_mint,
        TOKEN_0_AMOUNT,
        TOKEN_1_AMOUNT,
        OPEN_TIME,
    )?;

    log::info!(
		"Initialization complete: {signature:?}, token_0_vault={}, token_1_vault={}, pool_state={}, pool_authority={}, lp_mint={}, creator_token_0={}, creator_token_1={}, creator_lp_ata={}", 
		init_keys.token_0_vault,
		init_keys.token_1_vault,
		init_keys.pool_state,
		init_keys.pool_authority,
		init_keys.lp_mint,
		init_keys.creator_token_0,
		init_keys.creator_token_1,
		init_keys.creator_lp_ata
	);

    let pool_liquidity = raydium.get_pool_liquidity(init_keys.pool_state)?;

    log::info!(
        "Current pool liquidity: token_0_amount={}, token_1_amount={}, lp_supply={}",
        pool_liquidity.token_0_amount,
        pool_liquidity.token_1_amount,
        pool_liquidity.lp_supply
    );

    // STEP 2: Add liquidity to the CP-AMM pool based on how many LP tokens we want to receive.

    const LP_TOKEN_AMOUNT: u64 = 10_000_000;

    let signature = raydium.add_liquidity(
        init_keys.pool_state,
        init_keys.pool_authority,
        init_keys.lp_mint,
        token_0_mint,
        token_1_mint,
        init_keys.token_0_vault,
        init_keys.token_1_vault,
        init_keys.creator_token_0,
        init_keys.creator_token_1,
        init_keys.creator_lp_ata,
        LP_TOKEN_AMOUNT,
    )?;

    log::info!("Added liquidity: {signature}");

    let pool_liquidity = raydium.get_pool_liquidity(init_keys.pool_state)?;

    log::info!(
        "Current pool liquidity: token_0_amount={}, token_1_amount={}, lp_supply={}",
        pool_liquidity.token_0_amount,
        pool_liquidity.token_1_amount,
        pool_liquidity.lp_supply
    );

    // STEP 3: Remove the same amount of liquidity from the CP-AMM pool.

    let signature = raydium.remove_liquidity(
        init_keys.pool_state,
        init_keys.pool_authority,
        init_keys.lp_mint,
        token_0_mint,
        token_1_mint,
        init_keys.token_0_vault,
        init_keys.token_1_vault,
        init_keys.creator_token_0,
        init_keys.creator_token_1,
        init_keys.creator_lp_ata,
        LP_TOKEN_AMOUNT,
    )?;

    log::info!("Removed liquidity: {signature}");

    let pool_liquidity = raydium.get_pool_liquidity(init_keys.pool_state)?;

    log::info!(
        "Current pool liquidity: token_0_amount={}, token_1_amount={}, lp_supply={}",
        pool_liquidity.token_0_amount,
        pool_liquidity.token_1_amount,
        pool_liquidity.lp_supply
    );

    // STEP 4: Add and remove liquidity from the CP-AMM pool in the same transaction.

    let signature = raydium.add_and_remove_liquidity(
        init_keys.pool_state,
        init_keys.pool_authority,
        init_keys.lp_mint,
        token_0_mint,
        token_1_mint,
        init_keys.token_0_vault,
        init_keys.token_1_vault,
        init_keys.creator_token_0,
        init_keys.creator_token_1,
        init_keys.creator_lp_ata,
        LP_TOKEN_AMOUNT,
    )?;

    log::info!("Added and removed liquidity in the same transaction: {signature}");

    let pool_liquidity = raydium.get_pool_liquidity(init_keys.pool_state)?;

    log::info!(
        "Current pool liquidity: token_0_amount={}, token_1_amount={}, lp_supply={}",
        pool_liquidity.token_0_amount,
        pool_liquidity.token_1_amount,
        pool_liquidity.lp_supply
    );

    Ok(())
}

/// Helper function used to order tokens when creating the CP-AMM pool.
fn order_tokens(token_a: Pubkey, token_b: Pubkey) -> (Pubkey, Pubkey) {
    if token_a < token_b {
        (token_a, token_b)
    } else {
        (token_b, token_a)
    }
}
