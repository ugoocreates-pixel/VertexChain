#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address,
    Env, Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Signers,
    Threshold,
    HighValueThreshold,
    NextTxId,
    PendingTx(u64),
    Approval(u64, Address),
    ApprovalCount(u64),
    Balance(Address),
}

#[derive(Clone)]
#[contracttype]
pub struct PendingTx {
    pub id: u64,
    pub from: Address,
    pub to: Address,
    pub amount: i128,
    pub payload: Symbol,
    pub asset: Option<Address>,
    pub created_at: u64,
    pub executed: bool,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MultiSigError {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    Unauthorized = 3,
    InvalidThreshold = 4,
    DuplicateSigner = 5,
    InvalidAmount = 6,
    PendingTxNotFound = 7,
    UnauthorizedSigner = 8,
    DuplicateApproval = 9,
    AlreadyExecuted = 10,
    InsufficientBalance = 11,
    MultisigNotConfigured = 12,
    Overflow = 13,
}

mod events;
pub use events::MultisigEvents;

pub fn initialize_state(env: &Env, admin: Address) {
    if env.storage().instance().has(&DataKey::Admin) {
        panic_with_error!(env, MultiSigError::AlreadyInitialized);
    }

    env.storage().instance().set(&DataKey::Admin, &admin);
    env.storage()
        .instance()
        .set(&DataKey::Signers, &Vec::<Address>::new(env));
    env.storage().instance().set(&DataKey::Threshold, &0u32);
    env.storage()
        .instance()
        .set(&DataKey::HighValueThreshold, &i128::MAX);
    env.storage().instance().set(&DataKey::NextTxId, &0u64);
}

pub fn get_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::NotInitialized))
}

pub fn require_admin(env: &Env, caller: &Address) {
    caller.require_auth();
    let admin = get_admin(env);
    if admin != caller.clone() {
        panic_with_error!(env, MultiSigError::Unauthorized);
    }
}

pub fn set_signers(env: &Env, caller: Address, signers: Vec<Address>, threshold: u32) {
    require_admin(env, &caller);
    validate_signer_config(env, &signers, threshold);

    env.storage().instance().set(&DataKey::Signers, &signers);
    env.storage()
        .instance()
        .set(&DataKey::Threshold, &threshold);
}

pub fn set_high_value_threshold(env: &Env, caller: Address, amount: i128) {
    require_admin(env, &caller);

    if amount < 0 {
        panic_with_error!(env, MultiSigError::InvalidAmount);
    }

    env.storage()
        .instance()
        .set(&DataKey::HighValueThreshold, &amount);
}

pub fn get_signers(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::Signers)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn get_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::Threshold)
        .unwrap_or(0)
}

pub fn get_high_value_threshold(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::HighValueThreshold)
        .unwrap_or(i128::MAX)
}

pub fn ensure_multisig_configured(env: &Env) {
    let signers = get_signers(env);
    let threshold = get_threshold(env);

    if signers.is_empty() || threshold == 0 || threshold > signers.len() {
        panic_with_error!(env, MultiSigError::MultisigNotConfigured);
    }
}

pub fn require_signer(env: &Env, signer: &Address) {
    if !is_signer(env, signer) {
        panic_with_error!(env, MultiSigError::UnauthorizedSigner);
    }
}

pub fn is_signer(env: &Env, signer: &Address) -> bool {
    let signers = get_signers(env);
    for configured in signers.iter() {
        if configured == signer.clone() {
            return true;
        }
    }
    false
}

pub fn next_tx_id(env: &Env) -> u64 {
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::NextTxId)
        .unwrap_or(0);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::Overflow));

    env.storage().instance().set(&DataKey::NextTxId, &next);
    next
}

pub fn has_approval(env: &Env, tx_id: u64, signer: &Address) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::Approval(tx_id, signer.clone()))
}

pub fn get_approval_count(env: &Env, tx_id: u64) -> u32 {
    env.storage()
        .persistent()
        .get(&DataKey::ApprovalCount(tx_id))
        .unwrap_or(0)
}

pub fn record_approval(env: &Env, tx_id: u64, signer: &Address) -> u32 {
    if has_approval(env, tx_id, signer) {
        panic_with_error!(env, MultiSigError::DuplicateApproval);
    }

    env.storage()
        .persistent()
        .set(&DataKey::Approval(tx_id, signer.clone()), &true);

    let current = get_approval_count(env, tx_id);
    let next = current
        .checked_add(1)
        .unwrap_or_else(|| panic_with_error!(env, MultiSigError::Overflow));

    env.storage()
        .persistent()
        .set(&DataKey::ApprovalCount(tx_id), &next);

    next
}

fn validate_signer_config(env: &Env, signers: &Vec<Address>, threshold: u32) {
    let signer_count = signers.len();

    if signers.is_empty() || threshold == 0 || threshold > signer_count {
        panic_with_error!(env, MultiSigError::InvalidThreshold);
    }

    for i in 0..signer_count {
        let signer_i = signers
            .get(i)
            .unwrap_or_else(|| panic_with_error!(env, MultiSigError::InvalidThreshold));

        for j in (i + 1)..signer_count {
            let signer_j = signers
                .get(j)
                .unwrap_or_else(|| panic_with_error!(env, MultiSigError::InvalidThreshold));
            if signer_i == signer_j {
                panic_with_error!(env, MultiSigError::DuplicateSigner);
            }
        }
    }
}

#[contract]
pub struct MultisigContract;

#[contractimpl]
impl MultisigContract {
    pub fn initialize(env: Env, admin: Address) {
        initialize_state(&env, admin);
    }

    pub fn get_admin(env: Env) -> Address {
        get_admin(&env)
    }

    pub fn set_signers(env: Env, caller: Address, signers: Vec<Address>, threshold: u32) {
        set_signers(&env, caller, signers, threshold);
    }

    pub fn set_high_value_threshold(env: Env, caller: Address, amount: i128) {
        set_high_value_threshold(&env, caller, amount);
    }

    pub fn get_signers(env: Env) -> Vec<Address> {
        get_signers(&env)
    }

    pub fn get_threshold(env: Env) -> u32 {
        get_threshold(&env)
    }

    pub fn get_high_value_threshold(env: Env) -> i128 {
        get_high_value_threshold(&env)
    }

    pub fn submit_transaction(
        env: Env,
        caller: Address,
        to: Address,
        amount: i128,
        payload: Symbol,
        asset: Option<Address>,
    ) -> u64 {
        require_signer(&env, &caller);
        ensure_multisig_configured(&env);

        let tx_id = next_tx_id(&env);
        let tx = PendingTx {
            id: tx_id,
            from: caller.clone(),
            to: to.clone(),
            amount,
            payload,
            asset,
            created_at: env.ledger().timestamp(),
            executed: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::PendingTx(tx_id), &tx);

        MultisigEvents::pending_created(&env, &tx);

        tx_id
    }

    pub fn approve_transaction(env: Env, caller: Address, tx_id: u64) -> u32 {
        require_signer(&env, &caller);

        let tx: PendingTx = env
            .storage()
            .persistent()
            .get(&DataKey::PendingTx(tx_id))
            .unwrap_or_else(|| panic_with_error!(&env, MultiSigError::PendingTxNotFound));

        if tx.executed {
            panic_with_error!(&env, MultiSigError::AlreadyExecuted);
        }

        let approvals_count = record_approval(&env, tx_id, &caller);
        let threshold = get_threshold(&env);

        MultisigEvents::approval_recorded(&env, tx_id, &caller, approvals_count, threshold);

        approvals_count
    }

    pub fn execute_transaction(env: Env, caller: Address, tx_id: u64) {
        require_signer(&env, &caller);

        let mut tx: PendingTx = env
            .storage()
            .persistent()
            .get(&DataKey::PendingTx(tx_id))
            .unwrap_or_else(|| panic_with_error!(&env, MultiSigError::PendingTxNotFound));

        if tx.executed {
            panic_with_error!(&env, MultiSigError::AlreadyExecuted);
        }

        let approvals_count = get_approval_count(&env, tx_id);
        let threshold = get_threshold(&env);

        if approvals_count < threshold {
            panic_with_error!(&env, MultiSigError::Unauthorized);
        }

        tx.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::PendingTx(tx_id), &tx);

        MultisigEvents::transaction_executed(&env, &tx, &caller);
    }

    pub fn get_transaction(env: Env, tx_id: u64) -> Option<PendingTx> {
        env.storage().persistent().get(&DataKey::PendingTx(tx_id))
    }

    pub fn get_approval_count(env: Env, tx_id: u64) -> u32 {
        get_approval_count(&env, tx_id)
    }

    pub fn has_approval(env: Env, tx_id: u64, signer: Address) -> bool {
        has_approval(&env, tx_id, &signer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_initialize() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(MultisigContract, ());
        let client = MultisigContractClient::new(&env, &contract_id);

        client.initialize(&admin);
        assert_eq!(client.get_admin(), admin);
    }

    #[test]
    // MultiSigError::AlreadyInitialized = 2 (`#[repr(u32)]`) — keep in sync if enum is reordered.
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_cannot_initialize_twice() {
        let env = Env::default();
        let admin = Address::generate(&env);

        let contract_id = env.register(MultisigContract, ());
        let client = MultisigContractClient::new(&env, &contract_id);

        client.initialize(&admin);
        client.initialize(&admin);
    }

    #[test]
    fn test_set_signers() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let signer1 = Address::generate(&env);
        let signer2 = Address::generate(&env);

        let contract_id = env.register(MultisigContract, ());
        let client = MultisigContractClient::new(&env, &contract_id);

        client.initialize(&admin);

        let mut signers = Vec::new(&env);
        signers.push_back(signer1.clone());
        signers.push_back(signer2.clone());

        client.set_signers(&admin, &signers, &2);

        let retrieved_signers = client.get_signers();
        assert_eq!(retrieved_signers.len(), 2);
        assert_eq!(client.get_threshold(), 2);
    }
}
