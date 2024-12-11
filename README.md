# Raydium Add/Remove liquidity

This program integrates with the Raydium CP-AMM and demonstrates four key steps:

- Step 1

Initialize a CP-AMM pool using previously created SPL tokens (details below).

If the pool is already initialized, the program returns information about the existing pool.

- Step 2

Add liquidity to the CP-AMM pool by specifying the desired number of LP tokens.

- Step 3

Remove liquidity from the CP-AMM pool by redeeming the same amount of LP tokens.

- Step 4

Perform both liquidity deposit and withdrawal in a single transaction.

The program constructs a sequence of instructions for adding and removing liquidity and combines them into one transaction.

# Program structure

The `RaydiumIntegration` struct implements the following methods:

```rust
/// Creates a new Raydium integration with initialized clients and payer wallet.
pub fn new(payer: Rc<Keypair>, cluster: Cluster) -> Result<Self> {...}

/// Initializes a new Raydium CP-AMM pool or returns data from an existing pool.
pub fn initialize_pool(&self, amm_config_key: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_amount: u64, token_1_amount: u64, open_time: u64) -> Result<(Option<Signature>, InitializationKeys)> {...}

/// Adds liquidity to a Raydium CP-AMM pool.
pub fn add_liquidity(&self, pool_state: Pubkey, pool_authority: Pubkey, lp_mint: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_vault: Pubkey, token_1_vault: Pubkey, owner_token_0: Pubkey, owner_token_1: Pubkey, owner_lp: Pubkey, lp_token_amount: u64) -> Result<Signature> {...}

/// Creates instructions for depositing liquidity to a Raydium CP-AMM pool.
fn create_deposit_instructions(&self, pool_state: Pubkey, pool_authority: Pubkey, lp_mint: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_vault: Pubkey, token_1_vault: Pubkey, owner_token_0: Pubkey, owner_token_1: Pubkey, owner_lp: Pubkey, lp_token_amount: u64) -> Result<Vec<Instruction>> {...}

/// Removes liquidity from a Raydium CP-AMM pool.
pub fn remove_liquidity(&self, pool_state: Pubkey, pool_authority: Pubkey, lp_mint: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_vault: Pubkey, token_1_vault: Pubkey, owner_token_0: Pubkey, owner_token_1: Pubkey, owner_lp: Pubkey, lp_token_amount: u64) -> Result<Signature> {...}

/// Creates instructions for withdrawing liquidity from a Raydium CP-AMM pool.
fn create_withdrawal_instructions(&self, pool_state: Pubkey, pool_authority: Pubkey, lp_mint: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_vault: Pubkey, token_1_vault: Pubkey, owner_token_0: Pubkey, owner_token_1: Pubkey, owner_lp: Pubkey, lp_token_amount: u64) -> Result<Vec<Instruction>> {...}

/// Adds and removes liquidity from a Raydium CP-AMM pool in a single transaction.
pub fn add_and_remove_liquidity(&self, pool_state: Pubkey, pool_authority: Pubkey, lp_mint: Pubkey, token_0_mint: Pubkey, token_1_mint: Pubkey, token_0_vault: Pubkey, token_1_vault: Pubkey, owner_token_0: Pubkey, owner_token_1: Pubkey, owner_lp: Pubkey, lp_token_amount: u64) -> Result<Signature> {...}

/// Dynamically calculate token amounts needed for deposit or expected from withdrawal.
fn calculate_token_amounts(&self, pool_state: Pubkey, lp_token_amount: u64, deposit: bool) -> Result<(u64, u64)> {...}

/// Lists available AMM configurations.
pub fn list_amm_configs(&self) -> Result<()> {...}

/// Returns an AMM configuration for the specified index if it exists.
pub fn get_amm_config_by_index(&self, index: u16) -> Result<(Pubkey, AmmConfig)> {...}

/// Fetches the current liquidity amounts from a Raydium CP-AMM pool.
pub fn get_pool_liquidity(&self, pool_state: Pubkey) -> Result<PoolLiquidity> {...}
```


# Program output

Example program output when initializing the pool for the first time:

```
[INFO] Initializing pool with tokens 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU (10000000) and 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho (10000000)
[32/32] Finalizing transaction 5HqDhpdAtMxFJ2pzE8HZ5VeNphZLN61VbyjkHjTr5epLhtBgYW3JnQN1uzmQqW39eSXPigZ2zGUx6KyR7ewJJHmU
[INFO] Initialization complete: Some(5HqDhpdAtMxFJ2pzE8HZ5VeNphZLN61VbyjkHjTr5epLhtBgYW3JnQN1uzmQqW39eSXPigZ2zGUx6KyR7ewJJHmU), token_0_vault=Fzh82ayt5anmjQvh7bm3MhgaiNZ4sui1jvYxLacFjMtE, token_1_vault=BBQ1zfuuBzgPkwT9gH3NjZvVUJfUFKYayNczm6puLzyj, pool_state=85cCgxAV8r3RYZKUWaFqrson2J4KLign9THRXbfEfq7G, pool_authority=7rQ1QFNosMkUCuh7Z7fPbTHvh73b68sQYdirycEzJVuw, lp_mint=Gv5QBxrkP2bUvUVfP4Gv3uTbG9j3AJkmGMB4zZ5F2BVB, creator_token_0=77JDsk2LWGFufEgqXeZBg1AvX1LkyCFaGPGWwNwFDviq, creator_token_1=3YbHpJ4JTDQ17dyJ843acLbHnzWRdMSNMz45PSXuhiJS, creator_lp_ata=VCiKWZvhAx5S56fBrHT2qxiZaFXNeQUdHCJ4q4x9bkv
[INFO] Current pool liquidity: token_0_amount=10000000, token_1_amount=10000000, lp_supply=10000000
[INFO] Added liquidity: 481CuJ5imBMMjVUetN1UbGxrWVHXhymaVA8cj7JYnh9LhJSVCf9xkX5ZnRq1ZBNh21Z6DHxC51tLASyBXc1AcCpH
[INFO] Current pool liquidity: token_0_amount=20000000, token_1_amount=20000000, lp_supply=20000000
[INFO] Removed liquidity: 3oWibqSL7cjSHx9Mtwq2Y465irwyDP9ksLPv76Zwhnqp6cDqEgpsPdW8iBM3uC5P1pfn8HU16DGvYDAMkwYo4Xcr
[INFO] Current pool liquidity: token_0_amount=10000000, token_1_amount=10000000, lp_supply=10000000
[INFO] Added and removed liquidity in the same transaction: 2GMWpYqfh415cfFJD6Sfv5RMs62VgC6WLJstbqeHzgspwYGD35Ez4ySqAvxma7HaSWU8GK6VHJgLRC7Fo8isTAP
[INFO] Current pool liquidity: token_0_amount=20000000, token_1_amount=20000000, lp_supply=20000000
```

Example program output once the pool has been initialized:

```
[INFO] AMM config 9zSzfkYy6awexsHvmggeH36pfVUdDGyCcwmjT3AQPBj6: index=0, trade_fee_rate=2500, protocol_fee_rate=120000
[INFO] Using AMM config with index: 0
[INFO] Pool already exists for tokens 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU and 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho
[INFO] Initialization complete: None, token_0_vault=Fzh82ayt5anmjQvh7bm3MhgaiNZ4sui1jvYxLacFjMtE, token_1_vault=BBQ1zfuuBzgPkwT9gH3NjZvVUJfUFKYayNczm6puLzyj, pool_state=85cCgxAV8r3RYZKUWaFqrson2J4KLign9THRXbfEfq7G, pool_authority=7rQ1QFNosMkUCuh7Z7fPbTHvh73b68sQYdirycEzJVuw, lp_mint=Gv5QBxrkP2bUvUVfP4Gv3uTbG9j3AJkmGMB4zZ5F2BVB, creator_token_0=77JDsk2LWGFufEgqXeZBg1AvX1LkyCFaGPGWwNwFDviq, creator_token_1=3YbHpJ4JTDQ17dyJ843acLbHnzWRdMSNMz45PSXuhiJS, creator_lp_ata=VCiKWZvhAx5S56fBrHT2qxiZaFXNeQUdHCJ4q4x9bkv
[INFO] Current pool liquidity: token_0_amount=10000000, token_1_amount=10000000, lp_supply=10000000
[INFO] Added liquidity: 481CuJ5imBMMjVUetN1UbGxrWVHXhymaVA8cj7JYnh9LhJSVCf9xkX5ZnRq1ZBNh21Z6DHxC51tLASyBXc1AcCpH
[INFO] Current pool liquidity: token_0_amount=20000000, token_1_amount=20000000, lp_supply=20000000
[INFO] Removed liquidity: 3oWibqSL7cjSHx9Mtwq2Y465irwyDP9ksLPv76Zwhnqp6cDqEgpsPdW8iBM3uC5P1pfn8HU16DGvYDAMkwYo4Xcr
[INFO] Current pool liquidity: token_0_amount=10000000, token_1_amount=10000000, lp_supply=10000000
[INFO] Added and removed liquidity in the same transaction: 2GMWpYqfh415cfFJD6Sfv5RMs62VgC6WLJstbqeHzgspwYGD35Ez4ySqAvxma7HaSWU8GK6VHJgLRC7Fo8isTAP
[INFO] Current pool liquidity: token_0_amount=20000000, token_1_amount=20000000, lp_supply=20000000
```

# Token A

Create SPL TokenA

```sh
spl-token create-token --decimals 6

Creating token 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho under program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

Address:  69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho
Decimals:  6

Signature: 2fkYupkTRbrXWdKrVUWcuedRWBuXffy26eKgDgdg4FHCwd5T6H7CRZ8CwFMUcCTBNJm3DC2dSJtTBWmuTB8yBH96
```

Create TokenA ATA:

```sh
spl-token create-account 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho

Creating account 3YbHpJ4JTDQ17dyJ843acLbHnzWRdMSNMz45PSXuhiJS

Signature: 3hRiLgcCrAxPxGKUHiPeMwtZkUrAX6cCxgYMeNXPPuKGMTrgAy4teWSXTTnv5qwWjyQNejrw3dxMiRXdQrK76mWT
```

Mint 10000000 tokens:

```sh
spl-token mint 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho 10000000

Minting 10000000 tokens
  Token: 69iigTreHjCuinTmPvbaVdKtvwVwkWp5nd8ERNqg49ho
  Recipient: 3YbHpJ4JTDQ17dyJ843acLbHnzWRdMSNMz45PSXuhiJS

Signature: UNuhyUAwxHLXdvk821aTNRCdKmGQ3BMhsEsX2Po3XAdH9Ywsn72UXHAoRA8k8q3dnTHW7wHMvUvNovU8Wd2RUYj
```

# Token B

Create SPL TokenA:

```sh
spl-token create-token --decimals 6

Creating token 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU under program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA

Address:  2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU
Decimals:  6

Signature: 5meBddTBteEL2bumxzRCztUbK9RkGVmWksZQdT2utEFbBh5DVJtN88LabAACvnbACKCkFZ8PE4GVk5PbvWQY9xqb
```

Create TokenA ATA:

```sh
spl-token create-account 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU

Creating account 77JDsk2LWGFufEgqXeZBg1AvX1LkyCFaGPGWwNwFDviq

Signature: sPshp86mZzSB4P7nnYZLu2LZQ7eLpGxGHBhVmFKcNMQCvh6D4sUkYcMfEEV97umwh4CAQVkzW9WJVWU5T3mJHCg
```

Mint 10000000 tokens:

```sh
spl-token mint 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU 10000000

Minting 10000000 tokens
  Token: 2vEyg5rDJZmSjsTKKGKVtETu2DSGyKwcs8h3kycLyziU
  Recipient: 77JDsk2LWGFufEgqXeZBg1AvX1LkyCFaGPGWwNwFDviq

Signature: 2ZbxNHc5Vu2g3p2ir7Z1CnFpxSVU7nqWPnhGuiWJDjA1D18rHddEJqSgk5mpS5fJ4wRiPEuTHghmHEWx3jUV7gRC
```

# Wallet file

The keypair file used for this project (`./devnet.json`) is git ignored, but can be provided if needed.
