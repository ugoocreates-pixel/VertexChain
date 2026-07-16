use soroban_sdk::{symbol_short, Address, Env, String};

pub struct GovernanceEvents;

impl GovernanceEvents {
    pub fn admin_updated(env: &Env, previous_admin: &Address, new_admin: &Address) {
        let topics = (symbol_short!("gov"), symbol_short!("admin"));
        env.events().publish(
            topics,
            (
                previous_admin.clone(),
                new_admin.clone(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn proposal_created(
        env: &Env,
        id: u32,
        proposer: &Address,
        config_key: &String,
        config_value: &String,
    ) {
        let topics = (symbol_short!("gov"), symbol_short!("created"));
        env.events().publish(
            topics,
            (
                id,
                proposer.clone(),
                config_key.clone(),
                config_value.clone(),
                env.ledger().timestamp(),
            ),
        );
    }

    pub fn voted(env: &Env, id: u32, voter: &Address) {
        let topics = (symbol_short!("gov"), symbol_short!("voted"));
        env.events()
            .publish(topics, (id, voter.clone(), env.ledger().timestamp()));
    }

    pub fn proposal_executed(env: &Env, id: u32, config_key: &String, config_value: &String) {
        let topics = (symbol_short!("gov"), symbol_short!("executed"));
        env.events().publish(
            topics,
            (
                id,
                config_key.clone(),
                config_value.clone(),
                env.ledger().timestamp(),
            ),
        );
    }
}
