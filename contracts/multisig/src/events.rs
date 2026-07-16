use crate::PendingTx;
use soroban_sdk::{symbol_short, Address, Env};

pub struct MultisigEvents;

impl MultisigEvents {
    pub fn pending_created(env: &Env, tx: &PendingTx) {
        let topics = (symbol_short!("tx"), symbol_short!("pending"), tx.id);
        env.events().publish(
            topics,
            (tx.from.clone(), tx.to.clone(), tx.amount, tx.asset.clone()),
        );
    }

    pub fn approval_recorded(
        env: &Env,
        tx_id: u64,
        signer: &Address,
        approvals_count: u32,
        threshold: u32,
    ) {
        let topics = (symbol_short!("approve"), symbol_short!("record"), tx_id);
        env.events()
            .publish(topics, (signer.clone(), approvals_count, threshold));
    }

    pub fn transaction_executed(env: &Env, tx: &PendingTx, executor: &Address) {
        let topics = (symbol_short!("tx"), symbol_short!("executed"), tx.id);
        env.events().publish(
            topics,
            (
                executor.clone(),
                tx.from.clone(),
                tx.to.clone(),
                tx.amount,
                tx.asset.clone(),
            ),
        );
    }
}
