#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Wallet(Address),
    NextWalletId,
    TotalBatches,
    TotalWalletsCreated,
}

#[derive(Clone)]
#[contracttype]
pub struct Wallet {
    pub id: u64,
    pub owner: Address,
    pub created_at: u64,
}

#[derive(Clone)]
#[contracttype]
pub enum WalletCreateResult {
    Success(Address),
    Failure(Address, u32),
}

#[derive(Clone)]
#[contracttype]
pub enum WalletRecoveryResult {
    Success(Address, Address),
    Failure(Address, Address, u32),
}

#[derive(Clone)]
#[contracttype]
pub struct WalletCreateRequest {
    pub owner: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct WalletRecoveryRequest {
    pub old_owner: Address,
    pub new_owner: Address,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchCreateResult {
    pub total_requests: u32,
    pub successful: u32,
    pub failed: u32,
    pub results: Vec<WalletCreateResult>,
}

#[derive(Clone)]
#[contracttype]
pub struct BatchRecoveryResult {
    pub total_requests: u32,
    pub successful: u32,
    pub failed: u32,
    pub results: Vec<WalletRecoveryResult>,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchWalletError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    WalletNotFound = 4,
    WalletAlreadyExists = 5,
    InvalidRecovery = 6,
    DuplicateWallet = 7,
    EmptyBatch = 8,
    Overflow = 9,
}

mod events;
pub use events::BatchWalletEvents;

pub fn initialize(env: &Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(env, BatchWalletError::AlreadyInitialized);
    }

    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage().instance().set(&DataKey::NextWalletId, &0u64);
    env.storage().instance().set(&DataKey::TotalBatches, &0u64);
    env.storage()
        .instance()
        .set(&DataKey::TotalWalletsCreated, &0u64);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, BatchWalletError::NotInitialized))
}

pub fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin = get_admin(env);
    if admin != caller.clone() {
        panic_with_error!(env, BatchWalletError::Unauthorized);
    }
}

pub fn set_admin(env: &Env, caller: Address, new_admin: Address) {
    require_admin(env, &caller);
    env.storage().instance().set(&DataKey::Admin, &new_admin);
}

pub fn get_wallet(env: &Env, owner: Address) -> Option<Wallet> {
    env.storage().persistent().get(&DataKey::Wallet(owner))
}

pub fn next_wallet_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextWalletId)
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, BatchWalletError::Overflow));

    env.storage().instance().set(&DataKey::NextWalletId, &next);
    next
}

pub fn increment_total_batches(env: &Env) {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::TotalBatches)
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, BatchWalletError::Overflow));

    env.storage().instance().set(&DataKey::TotalBatches, &next);
}

pub fn increment_total_wallets_created(env: &Env) {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::TotalWalletsCreated)
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, BatchWalletError::Overflow));

    env.storage()
        .instance()
        .set(&DataKey::TotalWalletsCreated, &next);
}

pub fn get_total_batches(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::TotalBatches)
        .unwrap_or(0)
}

pub fn get_total_wallets_created(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::TotalWalletsCreated)
        .unwrap_or(0)
}

#[contract]
pub struct BatchWalletContract;

#[contractimpl]
impl BatchWalletContract {
    pub fn initialize(env: Env, admin: Address) {
        initialize(&env, admin);
    }

    pub fn get_admin(env: Env) -> Address {
        get_admin(&env)
    }

    pub fn set_admin(env: Env, caller: Address, new_admin: Address) {
        set_admin(&env, caller, new_admin);
    }

    pub fn get_wallet(env: Env, owner: Address) -> Option<Wallet> {
        get_wallet(&env, owner)
    }

    pub fn get_total_batches(env: Env) -> u64 {
        get_total_batches(&env)
    }

    pub fn get_total_wallets_created(env: Env) -> u64 {
        get_total_wallets_created(&env)
    }

    pub fn batch_create_wallets(
        env: Env,
        caller: Address,
        requests: Vec<WalletCreateRequest>,
    ) -> BatchCreateResult {
        require_admin(&env, &caller);

        if requests.is_empty() {
            panic_with_error!(&env, BatchWalletError::EmptyBatch);
        }

        let batch_id = get_total_batches(&env) + 1;
        BatchWalletEvents::batch_started(&env, batch_id, requests.len());

        let mut results = Vec::new(&env);
        let mut successful: u32 = 0;
        let mut failed: u32 = 0;

        // Check for duplicates within the batch
        let mut seen_addresses = Vec::new(&env);
        for i in 0..requests.len() {
            let request = requests.get(i).unwrap();
            let owner = request.owner.clone();

            // Check if this address was already seen in this batch
            for seen in seen_addresses.iter() {
                if seen == owner {
                    BatchWalletEvents::wallet_duplicate(&env, &owner);
                    panic_with_error!(&env, BatchWalletError::DuplicateWallet);
                }
            }
            seen_addresses.push_back(owner);
        }

        // Process each request
        for i in 0..requests.len() {
            let request = requests.get(i).unwrap();
            let owner = request.owner.clone();

            if let Some(_existing_wallet) = get_wallet(&env, owner.clone()) {
                results.push_back(WalletCreateResult::Failure(
                    owner.clone(),
                    1, // Already exists error code
                ));
                failed += 1;
            } else {
                let wallet_id = next_wallet_id(&env);
                let wallet = Wallet {
                    id: wallet_id,
                    owner: owner.clone(),
                    created_at: env.ledger().timestamp(),
                };

                env.storage()
                    .persistent()
                    .set(&DataKey::Wallet(owner.clone()), &wallet);

                BatchWalletEvents::wallet_created(&env, wallet_id, &owner);
                results.push_back(WalletCreateResult::Success(owner));
                successful += 1;
                increment_total_wallets_created(&env);
            }
        }

        increment_total_batches(&env);
        BatchWalletEvents::batch_completed(&env, batch_id, successful, failed);

        BatchCreateResult {
            total_requests: requests.len(),
            successful,
            failed,
            results,
        }
    }

    pub fn batch_recover_wallets(
        env: Env,
        caller: Address,
        requests: Vec<WalletRecoveryRequest>,
    ) -> BatchRecoveryResult {
        require_admin(&env, &caller);

        if requests.is_empty() {
            panic_with_error!(&env, BatchWalletError::EmptyBatch);
        }

        let batch_id = get_total_batches(&env) + 1;
        BatchWalletEvents::batch_started(&env, batch_id, requests.len());

        let mut results = Vec::new(&env);
        let mut successful: u32 = 0;
        let mut failed: u32 = 0;

        for i in 0..requests.len() {
            let request = requests.get(i).unwrap();
            let old_owner = request.old_owner.clone();
            let new_owner = request.new_owner.clone();

            // Check if old wallet exists
            let old_wallet = get_wallet(&env, old_owner.clone());
            if old_wallet.is_none() {
                results.push_back(WalletRecoveryResult::Failure(
                    old_owner.clone(),
                    new_owner.clone(),
                    1, // Old wallet not found
                ));
                failed += 1;
                continue;
            }

            // Check if new owner already has a wallet
            if let Some(_existing) = get_wallet(&env, new_owner.clone()) {
                results.push_back(WalletRecoveryResult::Failure(
                    old_owner.clone(),
                    new_owner.clone(),
                    2, // New wallet already exists
                ));
                failed += 1;
                continue;
            }

            // Check if old and new are the same
            if old_owner == new_owner {
                results.push_back(WalletRecoveryResult::Failure(
                    old_owner.clone(),
                    new_owner.clone(),
                    3, // Invalid recovery (same address)
                ));
                failed += 1;
                continue;
            }

            // Perform recovery
            let wallet = old_wallet.unwrap();
            env.storage()
                .persistent()
                .remove(&DataKey::Wallet(old_owner.clone()));

            let new_wallet = Wallet {
                id: wallet.id,
                owner: new_owner.clone(),
                created_at: wallet.created_at,
            };

            env.storage()
                .persistent()
                .set(&DataKey::Wallet(new_owner.clone()), &new_wallet);

            BatchWalletEvents::wallet_recovered(&env, &old_owner, &new_owner);
            results.push_back(WalletRecoveryResult::Success(old_owner, new_owner));
            successful += 1;
        }

        increment_total_batches(&env);
        BatchWalletEvents::batch_completed(&env, batch_id, successful, failed);

        BatchRecoveryResult {
            total_requests: requests.len(),
            successful,
            failed,
            results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::{Address, Env, Vec};

    fn setup_test_env() -> (Env, Address, BatchWalletContractClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.sequence_number = 12345;
        });

        let contract_id = env.register(BatchWalletContract, ());
        let client = BatchWalletContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin);

        (env, admin, client)
    }

    fn create_wallet_request(_env: &Env, owner: Address) -> WalletCreateRequest {
        WalletCreateRequest { owner }
    }

    fn create_recovery_request(
        _env: &Env,
        old_owner: Address,
        new_owner: Address,
    ) -> WalletRecoveryRequest {
        WalletRecoveryRequest {
            old_owner,
            new_owner,
        }
    }

    #[test]
    fn test_initialize_contract() {
        let (_env, admin, client) = setup_test_env();

        assert_eq!(client.get_admin(), admin);
        assert_eq!(client.get_total_batches(), 0);
        assert_eq!(client.get_total_wallets_created(), 0);
    }

    #[test]
    // BatchWalletError::AlreadyInitialized = 2 (`#[repr(u32)]`) — keep in sync if enum is reordered.
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_cannot_initialize_twice() {
        let (env, _admin, client) = setup_test_env();

        let new_admin = Address::generate(&env);
        client.initialize(&new_admin);
    }

    #[test]
    fn test_batch_create_wallets_single() {
        let (env, admin, client) = setup_test_env();

        let owner = Address::generate(&env);

        let mut requests: Vec<WalletCreateRequest> = Vec::new(&env);
        requests.push_back(create_wallet_request(&env, owner.clone()));

        let result = client.batch_create_wallets(&admin, &requests);

        assert_eq!(result.total_requests, 1);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 0);
        assert_eq!(result.results.len(), 1);

        let wallet = client.get_wallet(&owner).unwrap();
        assert_eq!(wallet.owner, owner);
        assert_eq!(wallet.id, 1);
    }

    #[test]
    fn test_batch_create_wallets_multiple() {
        let (env, admin, client) = setup_test_env();

        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let owner3 = Address::generate(&env);

        let mut requests: Vec<WalletCreateRequest> = Vec::new(&env);
        requests.push_back(create_wallet_request(&env, owner1.clone()));
        requests.push_back(create_wallet_request(&env, owner2.clone()));
        requests.push_back(create_wallet_request(&env, owner3.clone()));

        let result = client.batch_create_wallets(&admin, &requests);

        assert_eq!(result.total_requests, 3);
        assert_eq!(result.successful, 3);
        assert_eq!(result.failed, 0);

        let wallet1 = client.get_wallet(&owner1).unwrap();
        assert_eq!(wallet1.id, 1);
        let wallet2 = client.get_wallet(&owner2).unwrap();
        assert_eq!(wallet2.id, 2);
        let wallet3 = client.get_wallet(&owner3).unwrap();
        assert_eq!(wallet3.id, 3);
    }

    #[test]
    fn test_batch_create_wallets_partial_failures() {
        let (env, admin, client) = setup_test_env();

        let owner1 = Address::generate(&env);
        let owner2 = Address::generate(&env);
        let owner3 = Address::generate(&env);

        let mut requests1: Vec<WalletCreateRequest> = Vec::new(&env);
        requests1.push_back(create_wallet_request(&env, owner1.clone()));
        requests1.push_back(create_wallet_request(&env, owner2.clone()));
        client.batch_create_wallets(&admin, &requests1);

        let mut requests2: Vec<WalletCreateRequest> = Vec::new(&env);
        requests2.push_back(create_wallet_request(&env, owner1.clone()));
        requests2.push_back(create_wallet_request(&env, owner2.clone()));
        requests2.push_back(create_wallet_request(&env, owner3.clone()));

        let result = client.batch_create_wallets(&admin, &requests2);

        assert_eq!(result.total_requests, 3);
        assert_eq!(result.successful, 1);
        assert_eq!(result.failed, 2);

        let wallet3 = client.get_wallet(&owner3).unwrap();
        assert_eq!(wallet3.id, 3);
    }

    #[test]
    // BatchWalletError::DuplicateWallet = 7 (`#[repr(u32)]`) — keep in sync if enum is reordered.
    #[should_panic(expected = "Error(Contract, #7)")]
    fn test_batch_create_wallets_duplicate_in_batch() {
        let (env, admin, client) = setup_test_env();

        let owner = Address::generate(&env);
        let owner2 = Address::generate(&env);

        let mut requests: Vec<WalletCreateRequest> = Vec::new(&env);
        requests.push_back(create_wallet_request(&env, owner.clone()));
        requests.push_back(create_wallet_request(&env, owner2.clone()));
        requests.push_back(create_wallet_request(&env, owner.clone()));

        client.batch_create_wallets(&admin, &requests);
    }

    #[test]
    fn test_batch_recover_wallets_single_success() {
        let (env, admin, client) = setup_test_env();

        let original_owner = Address::generate(&env);
        let new_owner = Address::generate(&env);

        let mut create_requests: Vec<WalletCreateRequest> = Vec::new(&env);
        create_requests.push_back(create_wallet_request(&env, original_owner.clone()));
        client.batch_create_wallets(&admin, &create_requests);

        let mut recovery_requests: Vec<WalletRecoveryRequest> = Vec::new(&env);
        recovery_requests.push_back(create_recovery_request(
            &env,
            original_owner.clone(),
            new_owner.clone(),
        ));

        let recover_result = client.batch_recover_wallets(&admin, &recovery_requests);

        assert_eq!(recover_result.total_requests, 1);
        assert_eq!(recover_result.successful, 1);
        assert_eq!(recover_result.failed, 0);

        let original_wallet = client.get_wallet(&original_owner);
        assert!(original_wallet.is_none());

        let recovered_wallet = client.get_wallet(&new_owner).unwrap();
        assert_eq!(recovered_wallet.owner, new_owner);
        assert_eq!(recovered_wallet.id, 1);
    }
}
