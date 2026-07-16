use soroban_sdk::{symbol_short, Address, Env};

pub struct BatchWalletEvents;

impl BatchWalletEvents {
    pub fn batch_started(env: &Env, batch_id: u64, request_count: u32) {
        let topics = (symbol_short!("batch"), symbol_short!("started"), batch_id);
        env.events().publish(topics, request_count);
    }

    pub fn wallet_created(env: &Env, wallet_id: u64, owner: &Address) {
        let topics = (symbol_short!("wallet"), symbol_short!("created"), wallet_id);
        env.events().publish(topics, owner.clone());
    }

    pub fn wallet_duplicate(env: &Env, owner: &Address) {
        let topics = (symbol_short!("wallet"), symbol_short!("duplicate"));
        env.events().publish(topics, owner.clone());
    }

    pub fn batch_completed(env: &Env, batch_id: u64, successful: u32, failed: u32) {
        let topics = (symbol_short!("batch"), symbol_short!("completed"), batch_id);
        env.events().publish(topics, (successful, failed));
    }

    pub fn wallet_recovered(env: &Env, old_owner: &Address, new_owner: &Address) {
        let topics = (symbol_short!("wallet"), symbol_short!("recovered"));
        env.events()
            .publish(topics, (old_owner.clone(), new_owner.clone()));
    }
}
