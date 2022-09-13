use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption, UnorderedMap};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::serde_json::Result;
use near_sdk::bs58;
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use std::string;

type LeaseId = String;
type TokenId = String;
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
enum LeaseState {
    Pending,
    Active,
    Expired,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseJson {
    contract_addr: AccountId,
    token_id: TokenId,
    borrower: AccountId,
    expiration: u64, // TODO: duration
    amount_near: i64,
}

//struct for keeping track of the lease conditions
#[derive(BorshDeserialize, BorshSerialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseCondition {
    contract_addr: AccountId,
    token_id: TokenId,
    owner_id: AccountId,
    borrower: AccountId,
    approval_id: u64,
    expiration: u64, // TODO: duration
    amount_near: i64,
    state: LeaseState,
}


#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    lease_map: UnorderedMap<LeaseId, LeaseCondition>, // (lending_id, lending)
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    LendingsKey,
}



/*
    trait that will be used as the callback from the NFT contract. When nft_approve is
    called, it will fire a cross contract call to this marketplace and this is the function
    that is invoked.
*/
trait NonFungibleTokenApprovalsReceiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    );
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner: owner_id,
            lease_map: UnorderedMap::new(StorageKey::LendingsKey),
        }
    }

    // TODO
    pub fn lending_accept(&mut self, lending_id: LeaseId) {}

    pub fn leases_by_owner(&self, account_id: AccountId) -> Vec<LeaseCondition> {
        let mut results: Vec<LeaseCondition> = vec![];
        for lease in self.lease_map.iter() {
            if lease.1.owner_id == account_id {
                results.push(lease.1)
            }
        }
        results
    }
}

//implementation of the trait
#[near_bindgen]
impl NonFungibleTokenApprovalsReceiver for Contract {
    /// where we add the sale because we know nft owner can only call nft_approve

    #[payable]
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    ) {
        //the lease conditions come from the msg field
        let lease_json: LeaseJson =
        near_sdk::serde_json::from_str(&msg).expect("Not valid lease data");
 
        // build lease condition from the parsed json
        let lease_condition: LeaseCondition = LeaseCondition {
            owner_id: owner_id,
            approval_id: approval_id,
            contract_addr: lease_json.contract_addr,
            token_id: lease_json.token_id,
            borrower: lease_json.borrower,
            expiration: lease_json.expiration,
            amount_near: lease_json.amount_near,
            state: LeaseState::Pending,
        };

        let seed = near_sdk::env::random_seed();
        let key = bs58::encode(seed).with_alphabet(bs58::Alphabet::BITCOIN).into_string();
        self.lease_map
            .insert(&key, &lease_condition);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    use super::*;

    const MINT_COST: u128 = 1000000000000000000000000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(0));
        testing_env!(context.build());
        let mut contract = Contract::new(accounts(1).into());

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(MINT_COST)
            .predecessor_account_id(accounts(0))
            .build());
    }
}
