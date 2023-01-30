use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap};
use near_sdk::{env, ext_contract, AccountId, Balance, near_bindgen, PanicOnDefault, PromiseResult, Promise, StorageUsage};
use near_sdk::json_types::U128;
use near_sdk::serde::Serialize;

pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise;

    /// Returns the total supply of the token in a decimal string representation.
    fn ft_total_supply(&self) -> U128;

    /// Returns the balance of the account. If the account doesn't exist must returns `"0"`.
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

#[ext_contract(ext_fungible_token_receiver)]
trait FungibleTokenReceiver {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> Promise;
}

#[ext_contract(ext_self)]
trait FungibleTokenResolverExt {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

trait FungibleTokenResolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[derive(Serialize, BorshDeserialize, BorshSerialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FungibleTokenMetadata {
    pub version: String,
    pub name: String,
    pub symbol: String,
    pub reference: String,
    pub reference_hash: [u8; 32],
    pub decimals: u8,
}

pub trait FungibleTokenMetadataProvider {
    fn ft_metadata(&self) -> FungibleTokenMetadata;
}

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.ft_metadata.clone()
    }
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    contract_addr: AccountId,

    /// AccountID -> Account balance.
    pub accounts: LookupMap<AccountId, Balance>,

    /// Total supply of the all token.
    pub total_supply: Balance,

    /// The storage size in bytes for one account.
    pub account_storage_usage: StorageUsage,

    pub ft_metadata: FungibleTokenMetadata
}

#[near_bindgen]
impl Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let sender_id = env::predecessor_account_id();
        assert_eq!(
            env::attached_deposit(),
            1,
            "Requires attached deposit of exactly 1 yoctoNEAR"
        );
        let amount = amount.into();
        self.internal_transfer(sender_id, receiver_id, amount, memo);
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    ) -> Promise {
        assert_eq!(
            env::attached_deposit(),
            1,
            "Requires attached deposit of exactly 1 yoctoNEAR"
        );
        let sender_id = env::predecessor_account_id();
        let amount = amount.into();
        self.internal_transfer(sender_id.clone(), receiver_id.clone(), amount, memo);
        // Initiating receiver's call and the callback
        return ext_fungible_token_receiver::ext(self.contract_addr.clone())
        .ft_on_transfer(
            sender_id.clone(),
            amount.into(),
            msg,
        )
        .then(ext_self::ext(receiver_id.clone()).ft_resolve_transfer(
            sender_id,
            receiver_id.into(),
            amount.into(),
        )).as_return();
    }

    fn ft_total_supply(&self) -> U128 {
        U128::from(self.total_supply)
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.accounts.get(&account_id).unwrap_or(0).into()
    }

    fn internal_deposit(&mut self, account_id: AccountId, amount: Balance) {
        let balance = self
            .accounts
            .get(&account_id)
            .unwrap();
        let new_balance = balance.checked_add(amount).unwrap();
        self.accounts.insert(&account_id, &new_balance);
    }

    fn internal_withdraw(&mut self, account_id: AccountId, amount: Balance) {
        let balance = self
            .accounts
            .get(&account_id)
            .unwrap();
        let new_balance = balance.checked_sub(amount).unwrap(); 
        self.accounts.insert(&account_id, &new_balance);
    }

    fn internal_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: Balance,
        memo: Option<String>,
    ) {
        self.internal_withdraw(sender_id.clone(), amount);
        self.internal_deposit(receiver_id.clone(), amount);
    }
}

#[near_bindgen]
impl FungibleTokenResolver for Contract {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let amount: Balance = amount.into();

        // Get the unused amount from the `ft_on_transfer` call result.
        let unused_amount = match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(unused_amount) = near_sdk::serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount.0)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        if unused_amount > 0 {
            let receiver_balance = self.accounts.get(&receiver_id).unwrap_or(0);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                self.accounts
                    .insert(&receiver_id, &(receiver_balance - refund_amount));

                if let Some(sender_balance) = self.accounts.get(&sender_id) {
                    self.accounts
                        .insert(&sender_id, &(sender_balance + refund_amount));
                    return (amount - refund_amount).into();
                } 
            }
        }
        amount.into() // TODO: i think this should be something else, how many were returned
    }
}