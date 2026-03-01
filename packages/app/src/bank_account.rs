use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::user_connection;
use crate::money::MoneyViewMode;
use crate::{Money, MoneyView};

pub type BankAccountId = helper::UId<BankAccount>;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BankAccount {
    pub id: BankAccountId,
    pub user_connection_id: user_connection::Id,
    pub own_money: Money,
    pub user_connection_money: Money,
}

impl BankAccount {
    pub fn new(user_connection_id: user_connection::Id, own_money: Money, user_connection_money: Money) -> Self {
        Self {
            id: BankAccountId::new(),
            user_connection_id,
            own_money,
            user_connection_money,
        }
    }
}

#[component]
pub fn BankAccountView(bank_account: BankAccount) -> Element {
    rsx! {
        div { class: "bank-account",
            div { class: "bank-account__title", "BankAccount" }
            div { class: "bank-account__row", "Id: {bank_account.id}" }
            div { class: "bank-account__row", "UserConnection.Id: {bank_account.user_connection_id}" }
            div { class: "bank-account__row",
                "Own amount: "
                MoneyView {
                    money: bank_account.own_money,
                    shown_amount: MoneyViewMode::Current,
                }
            }
            div { class: "bank-account__row",
                "UserConnection amount: "
                MoneyView {
                    money: bank_account.user_connection_money,
                    shown_amount: MoneyViewMode::Current,
                }
            }
        }
    }
}