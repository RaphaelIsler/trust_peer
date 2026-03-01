use anyhow::{bail, Result};
use blockchain::Blockchain;

use crate::block_entry::BlockEntry;
use crate::money::Money;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct MoneyValidationState {
    last_amount: Option<Money>,
    snapshots_verified: usize,
}

impl MoneyValidationState {
    pub fn last_amount(&self) -> Option<Money> {
        self.last_amount
    }

    pub fn snapshots_verified(&self) -> usize {
        self.snapshots_verified
    }
}

pub fn verify_money_with_state(chain: &Blockchain<BlockEntry>) -> Result<MoneyValidationState> {
    chain.verify_with_state(MoneyValidationState::default(), |state, entry| {
        if let BlockEntry::CurrentAmount { money } = entry {
            if let Some(previous) = state.last_amount {
                if !(*money).is_projection_of(previous) {
                    bail!("invalid money progression: CurrentAmount is not a valid projection of previous state");
                }
            }

            state.last_amount = Some(*money);
            state.snapshots_verified += 1;
        }

        Ok(())
    })
}
