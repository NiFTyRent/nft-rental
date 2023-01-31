use std::collections::HashMap;

pub mod nft;
pub use crate::nft::metadata::*;

use near_contract_standards::non_fungible_token::TokenId;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    bs58, ext_contract, is_promise_success, promise_result_as_success, require, serde_json,
    CryptoHash,
};
use near_sdk::{
    env, log, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault, Promise, PromiseOrValue
};

pub mod externals;
mod utils;
pub use crate::externals::*;

// Copied from Paras market contract. Will need to be fine-tuned.
// https://github.com/ParasHQ/paras-marketplace-contract/blob/2dcb9e8b3bc8b9d4135d0f96f0255cd53116a6b4/paras-marketplace-contract/src/lib.rs#L17
pub const TGAS: u64 = 1_000_000_000_000;
pub const XCC_GAS: Gas = Gas(5 * TGAS); // cross contract gas
pub const GAS_FOR_NFT_TRANSFER: Gas = Gas(20_000_000_000_000);
pub const BASE_GAS: Gas = Gas(5 * TGAS);
pub const GAS_FOR_ROYALTIES: Gas = Gas(BASE_GAS.0 * 10u64);
pub const GAS_FOR_RESOLVE_CLAIM_BACK: Gas = Gas(BASE_GAS.0 * 10u64);
// the tolerance of lease price minus the sum of payout
// Set it to 1 to avoid linter error
pub const PAYOUT_DIFF_TORLANCE_YACTO: u128 = 1;
pub const MAX_LEN_PAYOUT: u32 = 50;

pub type LeaseId = String;
pub type PayoutHashMap = HashMap<AccountId, U128>;

/// A mapping of NEAR accounts to the amount each should be paid out, in
/// the event of a token-sale. The payout mapping MUST be shorter than the
/// maximum length specified by the financial contract obtaining this
/// payout data. Any mapping of length 10 or less MUST be accepted by
/// financial contracts, so 10 is a safe upper limit.
/// See more: https://nomicon.io/Standards/Tokens/NonFungibleToken/Payout#reference-implementation
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Payout {
    pub payout: PayoutHashMap,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, PartialEq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum LeaseState {
    Pending,
    Active,
    // TODO(libo): Expired is not ever been used. Clean it up.
    Expired,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseJson {
    contract_addr: AccountId,
    token_id: TokenId,
    borrower_id: AccountId,
    ft_contract_addr: AccountId,
    expiration: u64, // TODO: duration
    price: U128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct NftOnTransferJson {
    lease_id: String,
}

/// Struct for keeping track of the lease conditions
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseCondition {
    pub contract_addr: AccountId,    // NFT contract
    pub token_id: TokenId,           // NFT token
    pub lender_id: AccountId,        // Owner of the NFT
    pub borrower_id: AccountId,      // Borrower of the NFT
    pub ft_contract_addr: AccountId, // the account id for the ft contract
    pub approval_id: u64,            // Approval from owner to lease
    pub expiration: u64,             // TODO: duration
    pub price: u128,                 // Proposed lease price
    pub payout: Option<Payout>,      // Payout info (e.g. for Royalty split)
    pub state: LeaseState,           // Current lease state
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContractV1 {
    owner: AccountId,
    lease_map: UnorderedMap<LeaseId, LeaseCondition>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner: AccountId,   // same owner for both lease contract and nft contract 
    lease_map: UnorderedMap<LeaseId, LeaseCondition>,
    lease_ids_by_lender: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_ids_by_borrower: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_id_by_contract_addr_and_token_id: LookupMap<(AccountId, TokenId), LeaseId>,

    // iou nft contract related fields
    pub token_ids_per_owner: LookupMap<AccountId, UnorderedSet<TokenId>>, // tokens ids from each owner
    pub token_metadata_by_id: UnorderedMap<TokenId, TokenMetadata>, // This will also be used to query all existing token ids
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    LendingsKey,
    LeaseIdsByLender,
    LeasesIdsByLenderInner { account_id_hash: CryptoHash },
    LeaseIdsByBorrower,
    LeaseIdsByBorrowerInner { account_id_hash: CryptoHash },
    LeaseIdByContractAddrAndTokenId,
    TokenIdsPerOwner,
    TokenIdsPerOwnerInner { account_id_hash: CryptoHash },
    TokenMetadataById,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseAcceptanceJson {
    lease_id: String,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            owner: owner_id,
            lease_map: UnorderedMap::new(StorageKey::LendingsKey),
            lease_ids_by_lender: LookupMap::new(StorageKey::LeaseIdsByLender),
            lease_ids_by_borrower: LookupMap::new(StorageKey::LeaseIdsByBorrower),
            lease_id_by_contract_addr_and_token_id: LookupMap::new(
                StorageKey::LeaseIdByContractAddrAndTokenId,
            ),
            // iou nft related fields
            token_ids_per_owner: LookupMap::new(StorageKey::TokenIdsPerOwner.try_to_vec().unwrap()),
            token_metadata_by_id: UnorderedMap::new(
                StorageKey::TokenMetadataById.try_to_vec().unwrap()
            )
        }
    }

    /// Note: This migration function will clear all existing leases.
    #[init(ignore_state)]
    pub fn migrate() -> Self {
        let prev: ContractV1 = env::state_read().expect("ERR_NOT_INITIALIZED");
        assert_eq!(
            env::predecessor_account_id(),
            prev.owner,
            "Only the owner can invoke the migration"
        );

        Self {
            owner: prev.owner,
            lease_map: UnorderedMap::new(StorageKey::LendingsKey),
            lease_ids_by_lender: LookupMap::new(StorageKey::LeaseIdsByLender),
            lease_ids_by_borrower: LookupMap::new(StorageKey::LeaseIdsByBorrower),
            lease_id_by_contract_addr_and_token_id: LookupMap::new(
                StorageKey::LeaseIdByContractAddrAndTokenId,
            ),
            token_ids_per_owner: LookupMap::new(StorageKey::TokenIdsPerOwner.try_to_vec().unwrap()),
            token_metadata_by_id: UnorderedMap::new(
                StorageKey::TokenMetadataById.try_to_vec().unwrap()
            )
        }
    }

    #[private]
    pub fn activate_lease(&mut self, lease_id: LeaseId) -> U128{
        require!(
            is_promise_success(),
            "NFT transfer failed, abort lease activation."
        );
        log!("Activating lease ({})", &lease_id);

        // TODO: avoid re-fetch lease condition
        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();

        let new_lease_condition = LeaseCondition {
            state: LeaseState::Active,
            ..lease_condition
        };
        self.lease_map.insert(&lease_id, &new_lease_condition);
        // TODO: currently we do not return any amount to the borrower, revisit this logic if necessary
        let unused_ammount: U128 = U128::from(0);
        return unused_ammount;
    }

    #[payable]
    pub fn claim_back(&mut self, lease_id: LeaseId) {
        // Function to allow a user to claim back the NFT and rent after a lease expired.

        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();

        // 1. check expire time
        assert!(
            lease_condition.expiration < env::block_timestamp(),
            "Lease has not expired yet!"
        );
        // 2. check state == active
        assert_eq!(
            lease_condition.state,
            LeaseState::Active,
            "Queried Lease is not active!"
        );

        // 3. only original lender or service contract owner can claim back from expried lease
        assert!(
            (lease_condition.lender_id == env::predecessor_account_id())
                || (self.owner == env::predecessor_account_id()),
            "Only original lender or service owner can claim back!"
        );

        // 4. transfer nft to owner
        ext_nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(5 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer(
                lease_condition.lender_id.clone(),
                lease_condition.token_id.clone(),
                None,
                None,
            )
            // 5. Pay the rent to lender and royalty to relevant parties. Finally remove the lease.
            .then(
                ext_self::ext(env::current_account_id())
                    .with_attached_deposit(0)
                    .with_static_gas(GAS_FOR_RESOLVE_CLAIM_BACK)
                    .resolve_claim_back(lease_id),
            );
    }

    #[private]
    pub fn resolve_claim_back(&mut self, lease_id: LeaseId) {
        // TODO: avoid re-fetch lease condition
        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();

        match lease_condition.payout {
            Some(payout) => {
                for (receiver_id, amount) in payout.payout {
                    self.internal_transfer_ft(lease_condition.ft_contract_addr.clone(), receiver_id, amount);
                }
            }
            None => {
                self.internal_transfer_ft(lease_condition.ft_contract_addr.clone(), lease_condition.lender_id, U128::from(lease_condition.price));
            }
        }

        self.internal_remove_lease(&lease_id);
    }

    // private function to transfer FT to receiver_id
    fn internal_transfer_ft(&self, ft_contract_addr: AccountId, receiver_id: AccountId, amount: U128) -> Promise {
        ext_ft_core::ext(ft_contract_addr)
            .with_static_gas(Gas(10 * TGAS))
            .with_attached_deposit(1)
            .ft_transfer(receiver_id, amount, None).as_return()
    }

    pub fn leases_by_owner(&self, account_id: AccountId) -> Vec<(String, LeaseCondition)> {
        let mut results: Vec<(String, LeaseCondition)> = vec![];

        let lease_ids = self
            .lease_ids_by_lender
            .get(&account_id)
            .unwrap_or(UnorderedSet::new(b"s"));
        for id in lease_ids.iter() {
            let lease_condition = self.lease_map.get(&id).unwrap();
            results.push((id, lease_condition));
        }

        return results;
    }

    pub fn leases_by_borrower(&self, account_id: AccountId) -> Vec<(String, LeaseCondition)> {
        let mut results: Vec<(String, LeaseCondition)> = vec![];

        let lease_ids = self
            .lease_ids_by_borrower
            .get(&account_id)
            .unwrap_or(UnorderedSet::new(b"s"));
        for id in lease_ids.iter() {
            let lease_condition = self.lease_map.get(&id).unwrap();
            results.push((id, lease_condition))
        }
        return results;
    }

    pub fn get_borrower_by_contract_and_token(
        &self,
        contract_id: AccountId,
        token_id: TokenId,
    ) -> Option<AccountId> {
        // return the current borrower of the NFTs
        // Only active lease has valid borrower

        let lease_id = self
            .lease_id_by_contract_addr_and_token_id
            .get(&(contract_id, token_id));

        if lease_id.is_none() {
            return None;
        } else {
            let lease_condition = self.lease_map.get(&lease_id.unwrap()).unwrap();
            if lease_condition.state == LeaseState::Active {
                // only active lease has valid borrower
                return Some(lease_condition.borrower_id);
            } else {
                return None;
            }
        }
    }

    pub fn proxy_func_calls(&self, contract_id: AccountId, method_name: String, args: String) {
        // proxy function to open accessible functions calls in a NFT contract during lease
        let promise = Promise::new(contract_id.clone());

        // TODO: allow the lend to define white list of method names.
        // unreachable methods in leased NFT contract
        assert_ne!(
            "nft_transfer", &method_name,
            "Calling method is not accessiable!"
        );
        assert_ne!(
            "nft_approve", &method_name,
            "Calling method is not accessiable!"
        );

        promise.function_call(
            method_name.clone(),
            args.into(),
            env::attached_deposit(),
            Gas(5 * TGAS),
        );
    }

    #[private]
    pub fn create_lease_with_payout(
        &mut self,
        contract_id: AccountId,
        token_id: TokenId,
        owner_id: AccountId,
        borrower_id: AccountId,
        ft_contract_addr: AccountId,
        expiration: u64,
        price: u128,
        approval_id: u64,
    ) {
        let mut optional_payout: Option<Payout> = None;
        // if NFT has implemented the `nft_payout` interface
        // then process the result and verify if sum of payout is close enough to the original price
        if is_promise_success() {
            optional_payout = promise_result_as_success().map(|value| {
                let payout = serde_json::from_slice::<Payout>(&value).unwrap();
                let payout_diff: u128 = price
                    .checked_sub(
                        payout
                            .payout
                            .values()
                            .map(|v| v.0)
                            .into_iter()
                            .sum::<u128>(),
                    )
                    .unwrap();
                assert!(
                    payout_diff <= PAYOUT_DIFF_TORLANCE_YACTO,
                    "The difference between the lease price and the sum of payout is too large"
                );
                payout
            });
        }

        // build lease condition from the parsed json
        let lease_condition: LeaseCondition = LeaseCondition {
            lender_id: owner_id.clone(),
            approval_id,
            contract_addr: contract_id,
            token_id: token_id,
            borrower_id: borrower_id,
            ft_contract_addr: ft_contract_addr,
            expiration: expiration,
            price: price,
            payout: optional_payout,
            state: LeaseState::Pending,
        };

        let seed = near_sdk::env::random_seed();
        let lease_id = bs58::encode(seed)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string();

        self.internal_insert_lease(&lease_id, &lease_condition);
    }

    // helper method to remove records of a lease
    fn internal_remove_lease(&mut self, lease_id: &LeaseId) {
        // check if a lease condition exist
        let lease_condition = self
            .lease_map
            .get(&lease_id)
            .expect("Input lease_id does not exist");

        // remove lease map record
        self.lease_map.remove(&lease_id);

        // remove from index by_lender
        let mut lease_set = self
            .lease_ids_by_lender
            .get(&lease_condition.lender_id)
            .unwrap();
        lease_set.remove(&lease_id);

        if lease_set.is_empty() {
            self.lease_ids_by_lender.remove(&lease_condition.lender_id);
        } else {
            self.lease_ids_by_lender
                .insert(&lease_condition.lender_id, &lease_set);
        }

        // remove from index by_borrower
        let mut lease_set = self
            .lease_ids_by_borrower
            .get(&lease_condition.borrower_id)
            .unwrap();
        lease_set.remove(&lease_id);

        if lease_set.is_empty() {
            self.lease_ids_by_borrower
                .remove(&lease_condition.borrower_id);
        } else {
            self.lease_ids_by_borrower
                .insert(&lease_condition.borrower_id, &lease_set);
        }

        // remove from index by_contract_addr_and_token_id
        self.lease_id_by_contract_addr_and_token_id
            .remove(&(lease_condition.contract_addr, lease_condition.token_id));
    }

    // helper method to insert a new lease and update all indices
    fn internal_insert_lease(&mut self, lease_id: &LeaseId, lease_condition: &LeaseCondition) {
        // insert into lease map
        self.lease_map.insert(&lease_id, &lease_condition);

        //update index for leases by lender. If there are none, create a new empty set
        let mut lease_ids_set = self
            .lease_ids_by_lender
            .get(&lease_condition.lender_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(
                    StorageKey::LeasesIdsByLenderInner {
                        // get a new unique prefix for the collection by hashing owner
                        account_id_hash: utils::hash_account_id(&lease_condition.lender_id),
                    }
                    .try_to_vec()
                    .unwrap(),
                )
            });
        lease_ids_set.insert(&lease_id);
        self.lease_ids_by_lender
            .insert(&lease_condition.lender_id, &lease_ids_set);

        // update index for leases by borrower. If there are none, create a new empty set
        let mut lease_ids_set = self
            .lease_ids_by_borrower
            .get(&lease_condition.borrower_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(
                    StorageKey::LeaseIdsByBorrowerInner {
                        // get a new unique prefix for the collection by hashing owner
                        account_id_hash: utils::hash_account_id(&lease_condition.borrower_id),
                    }
                    .try_to_vec()
                    .unwrap(),
                )
            });
        lease_ids_set.insert(&lease_id);
        self.lease_ids_by_borrower
            .insert(&lease_condition.borrower_id, &lease_ids_set);

        // update index for lease_id_by_contract_addr_and_token_id
        self.lease_id_by_contract_addr_and_token_id.insert(
            &(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone(),
            ),
            &lease_id,
        );
    }
}

// TODO: move nft callback function to separate file e.g. nft_callbacks.rs
/**
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

        assert_eq!(token_id, lease_json.token_id);

        ext_nft::ext(lease_json.contract_addr.clone())
            .nft_payout(
                lease_json.token_id.clone(),    // token_id
                U128::from(lease_json.price.0), // price
                Some(MAX_LEN_PAYOUT),           // max_len_payout
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_attached_deposit(0)
                    .with_static_gas(GAS_FOR_ROYALTIES)
                    .create_lease_with_payout(
                        lease_json.contract_addr,
                        lease_json.token_id,
                        owner_id,
                        lease_json.borrower_id,
                        lease_json.ft_contract_addr,
                        lease_json.expiration,
                        lease_json.price.0,
                        approval_id,
                    ),
            )
            .as_return();
    }
}


/*
    The trait for receiving FT payment
    Depending on the FT contract implementation, it may need the users to register to deposit.
    So far we do not check if all partis have registered thier account on the FT contract,
        - Lender: he should make sure he has registered otherwise he will not receive the payment
        - Borrower: he cannot accept the lease if he does not register
        - Royalty payments: if any accounts in the royalty didn't register, they will not receive the payout. That
                             part of payment will be kept in this smart contract
*/
#[ext_contract(ext_ft_receiver)]
pub trait FungibleTokenReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}
#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// where we add the sale because we know nft owner can only call nft_approve
    #[payable]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        //the lease conditions come from the msg field
        let lease_acceptance_json: LeaseAcceptanceJson =
            near_sdk::serde_json::from_str(&msg).expect("Not valid lease data");

        // Borrower can accept a pending lending. When this happened, the lease contract does the following:
        // 1. Retrieve the lease data from the lease_map
        // 2. Check if the tx sender is the borrower
        // 2. Check if the FT contract is designated by the lender
        // 3. Check if the deposit equals rent
        // 4. Transfer the NFT to the lease contract
        // 5. Update the lease state, when transfer succeeds

        // TODO: check if the FT contract is the designated one
        let lease_condition: LeaseCondition = self.lease_map.get(&lease_acceptance_json.lease_id.clone()).unwrap();
        assert_eq!(
            lease_condition.borrower_id, sender_id,
            "Borrower is not the same one!"
        );
        assert_eq!(
            lease_condition.ft_contract_addr, env::predecessor_account_id(),
            "The FT contract address does match the lender's ask!"
        );
        assert_eq!(
            amount.0, lease_condition.price,
            "Deposit does not equal to the agreed rent!"
        );
        assert_eq!(
            lease_condition.state,
            LeaseState::Pending,
            "This lease is not pending on acceptance!"
        );

        ext_nft::ext(lease_condition.contract_addr.clone())
            .with_static_gas(Gas(10 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer(
                env::current_account_id(),                 // receiver_id
                lease_condition.token_id.clone(),          // token_id
                Some(lease_condition.approval_id.clone()), // approval_id
                None,                                      // memo
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_attached_deposit(0)
                    .with_static_gas(GAS_FOR_ROYALTIES)
                    .activate_lease(lease_acceptance_json.lease_id.clone()),
            )
            .as_return()
            .into()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    /*
    Unit test cases and helper functions

    Test naming format for better readability:
    - test_{function_name} _{succeeds_or_fails} _{condition}
    - When more than one test cases are needed for one function,
    follow the code order of testing failing conditions first and success condition last
    */
    use super::*;
    use near_sdk::serde_json::json;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig, VMConfig, ONE_NEAR};

    #[test]
    fn test_new() {
        let contract = Contract::new(accounts(1).into());
        assert_eq!(accounts(1), contract.owner);
        assert!(UnorderedMap::is_empty(&contract.lease_map));
    }

    #[test]
    fn test_activate_lease_with_payout_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();
        let payout = Payout {
            payout: HashMap::from([
                (accounts(2).into(), U128::from(1)),
                (accounts(3).into(), U128::from(4)),
            ]),
        };

        lease_condition.payout = Some(payout.clone());

        contract.lease_map.insert(&key, &lease_condition);
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(lease_condition.borrower_id.clone())
                .attached_deposit(lease_condition.price)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(Vec::new())],
        );

        contract.activate_lease(key.clone());

        let lease_condition_result = contract.lease_map.get(&key).unwrap();
        assert_eq!(lease_condition_result.payout, Some(payout));
        assert_eq!(lease_condition_result.state, LeaseState::Active);
    }

    #[test]
    fn test_activate_lease_without_payout_success() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();

        contract.lease_map.insert(&key, &lease_condition);
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(lease_condition.borrower_id.clone())
                .attached_deposit(lease_condition.price)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(Vec::new())],
        );

        contract.activate_lease(key.clone());

        let lease_condition_result = contract.lease_map.get(&key).unwrap();
        assert_eq!(lease_condition_result.payout, None);
        assert_eq!(lease_condition_result.state, LeaseState::Active);
    }

    #[test]
    #[should_panic]
    fn test_activate_lease_promise_panic() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(lease_condition.borrower_id.clone())
                .attached_deposit(lease_condition.price)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Failed],
        );

        contract.activate_lease(key.clone());

        let lease_condition_result = contract.lease_map.get(&key).unwrap();
        assert_eq!(lease_condition_result.payout, None);
        assert_eq!(lease_condition_result.state, LeaseState::Pending);
    }

    #[test]
    #[should_panic(expected = "Lease has not expired yet!")]
    fn test_claim_back_not_expired_yet() {
        let mut contract = Contract::new(accounts(1).into());

        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.expiration = 1000;

        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.expiration - 1)
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic(expected = "Only original lender or service owner can claim back!")]
    fn test_claim_back_wrong_lender() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        let key = "test_key".to_string();

        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(5).into()) // non-owner, non-lender
            .block_timestamp(lease_condition.expiration + 1)
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic(expected = "Queried Lease is not active!")]
    fn test_claim_back_inactive_lease() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Pending;
        let key = "test_key".to_string();

        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.expiration + 1)
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic]
    fn test_claim_back_non_exsisting_lease_id() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.expiration + 1)
            .build());

        let non_existing_key = "dummy_key".to_string();
        contract.claim_back(non_existing_key);
    }

    #[test]
    fn test_claim_back_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.price = 20;
        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.expiration + 1)
            .build());

        contract.claim_back(key);

        // Nothing can be checked, except the fact the call doesn't panic.
    }

    #[test]
    fn test_create_lease_with_payout_success() {
        let mut contract = Contract::new(accounts(1).into());
        let nft_contract_id: AccountId = accounts(4).into();
        let token_id: TokenId = "test_token".to_string();
        let owner_id: AccountId = accounts(2).into();
        let borrower_id: AccountId = accounts(3).into();
        let ft_contract_addr: AccountId = accounts(4).into();
        let price: u128 = 5;

        let payout = Payout {
            payout: HashMap::from([
                (accounts(2).into(), U128::from(1)),
                (accounts(3).into(), U128::from(4)),
            ]),
        };

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(borrower_id.clone())
                .attached_deposit(price)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(
                serde_json::to_vec(&payout).unwrap()
            )],
        );

        contract.create_lease_with_payout(
            nft_contract_id.clone(),
            token_id.clone(),
            owner_id.clone(),
            borrower_id.clone(),
            ft_contract_addr,
            1000,
            price,
            1,
        );

        assert!(!contract.lease_map.is_empty());
        let lease_condition = &contract.leases_by_owner(owner_id.clone())[0].1;

        assert_eq!(nft_contract_id, lease_condition.contract_addr);
        assert_eq!(token_id, lease_condition.token_id);
        assert_eq!(owner_id, lease_condition.lender_id);
        assert_eq!(borrower_id, lease_condition.borrower_id);
        assert_eq!(5, lease_condition.price);
        assert_eq!(1000, lease_condition.expiration);
        assert_eq!(Some(payout), lease_condition.payout);
    }

    #[test]
    #[should_panic(
        expected = "The difference between the lease price and the sum of payout is too large"
    )]
    fn test_create_lease_with_payout_failed_invalid_payout() {
        let mut contract = Contract::new(accounts(1).into());
        let nft_contract_id: AccountId = accounts(4).into();
        let token_id: TokenId = "test_token".to_string();
        let owner_id: AccountId = accounts(2).into();
        let borrower_id: AccountId = accounts(3).into();
        let ft_contract_addr: AccountId = accounts(4).into();
        let price: u128 = 5;

        let payout = Payout {
            payout: HashMap::from([
                (accounts(2).into(), U128::from(1)),
                (accounts(3).into(), U128::from(2)),
            ]),
        };

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(borrower_id.clone())
                .attached_deposit(price)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(
                serde_json::to_vec(&payout).unwrap()
            )],
        );

        contract.create_lease_with_payout(
            nft_contract_id.clone(),
            token_id.clone(),
            owner_id.clone(),
            borrower_id.clone(),
            ft_contract_addr,
            1000,
            price,
            1,
        );
    }

    #[test]
    fn test_get_borrower_by_contract_and_token_success_no_matching_borrower() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_borrower_id: AccountId = accounts(3).into();

        lease_condition.state = LeaseState::Active;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        let test_contract_id: AccountId = accounts(5).into();
        let test_token_id = "dummy_token".to_string();

        let result_borrower = contract.get_borrower_by_contract_and_token(
            test_contract_id.clone(),
            expected_token_id.clone(),
        );
        assert!(result_borrower.is_none());

        let result_borrower = contract.get_borrower_by_contract_and_token(
            expected_contract_address.clone(),
            test_token_id.clone(),
        );
        assert!(result_borrower.is_none());
    }

    #[test]
    fn test_get_borrower_by_contract_and_token_success_lease_is_inactive() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_borrower_id: AccountId = accounts(3).into();

        lease_condition.state = LeaseState::Pending;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        let result_borrower = contract
            .get_borrower_by_contract_and_token(expected_contract_address, expected_token_id);
        assert!(result_borrower.is_none());
    }

    #[test]
    fn test_get_borrower_by_contract_and_token_success_found_matching_borrower() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_borrower_id: AccountId = accounts(3).into();

        lease_condition.state = LeaseState::Active;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        let result_borrower = contract
            .get_borrower_by_contract_and_token(expected_contract_address, expected_token_id)
            .unwrap();
        assert!(result_borrower == expected_borrower_id);
    }

    #[test]
    fn test_leases_by_borrower_success() {
        let mut contract = Contract::new(accounts(1).into());
        let expected_borrower_id: AccountId = accounts(3).into();

        let mut lease_condition_1 = create_lease_condition_default();
        lease_condition_1.state = LeaseState::Active;
        lease_condition_1.token_id = "test_token_1".to_string();
        lease_condition_1.borrower_id = expected_borrower_id.clone();

        let key_1 = "test_key_1".to_string();
        contract.internal_insert_lease(&key_1, &lease_condition_1);

        let mut lease_condition_2 = create_lease_condition_default();
        lease_condition_2.state = LeaseState::Active;
        lease_condition_2.token_id = "test_token_2".to_string();
        lease_condition_2.borrower_id = expected_borrower_id.clone();

        let key_2 = "test_key_2".to_string();
        contract.internal_insert_lease(&key_2, &lease_condition_2);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition_1.lender_id.clone())
            .block_timestamp(lease_condition_1.expiration + 1)
            .build());

        let result = contract.leases_by_borrower(expected_borrower_id.clone());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_leases_by_owner_success() {
        let mut contract = Contract::new(accounts(1).into());
        let expected_owner_id: AccountId = accounts(2).into();

        let mut lease_condition_1 = create_lease_condition_default();
        lease_condition_1.state = LeaseState::Active;
        lease_condition_1.token_id = "test_token_1".to_string();
        lease_condition_1.lender_id = expected_owner_id.clone();

        let key_1 = "test_key_1".to_string();
        contract.internal_insert_lease(&key_1, &lease_condition_1);

        let mut lease_condition_2 = create_lease_condition_default();
        lease_condition_2.state = LeaseState::Active;
        lease_condition_2.token_id = "test_token_2".to_string();
        lease_condition_2.lender_id = expected_owner_id.clone();

        let key_2 = "test_key_2".to_string();
        contract.internal_insert_lease(&key_2, &lease_condition_2);

        let mut builder = VMContextBuilder::new();
        testing_env!(builder
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition_1.lender_id.clone())
            .block_timestamp(lease_condition_1.expiration + 1)
            .build());

        let result = contract.leases_by_owner(expected_owner_id.clone());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_internal_insert_lease_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.price = 20;
        lease_condition.contract_addr = accounts(4).into();
        lease_condition.token_id = "test_token".to_string();
        let key = "test_key".to_string();

        assert!(contract.lease_map.is_empty());
        assert!(!contract
            .lease_ids_by_borrower
            .contains_key(&lease_condition.borrower_id));
        assert!(!contract
            .lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));
        assert!(!contract
            .lease_id_by_contract_addr_and_token_id
            .contains_key(&(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone()
            )));

        contract.internal_insert_lease(&key, &lease_condition);

        assert!(contract.lease_map.len() == 1);
        assert!(contract
            .lease_ids_by_borrower
            .contains_key(&lease_condition.borrower_id));
        assert!(contract
            .lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));
        assert!(contract
            .lease_id_by_contract_addr_and_token_id
            .contains_key(&(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone()
            )));
    }

    #[test]
    fn test_internal_remove_lease_success_only_one_lease() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.price = 20;
        lease_condition.contract_addr = accounts(4).into();
        lease_condition.token_id = "test_token".to_string();
        let key = "test_key".to_string();

        contract.internal_insert_lease(&key, &lease_condition);

        assert!(contract.lease_map.len() == 1);
        assert!(contract
            .lease_ids_by_borrower
            .contains_key(&lease_condition.borrower_id));
        assert!(contract
            .lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));
        assert!(contract
            .lease_id_by_contract_addr_and_token_id
            .contains_key(&(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone()
            )));

        contract.internal_remove_lease(&key);

        assert!(contract.lease_map.is_empty());
        assert!(!contract
            .lease_ids_by_borrower
            .contains_key(&lease_condition.borrower_id));
        assert!(!contract
            .lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));
        assert!(!contract
            .lease_id_by_contract_addr_and_token_id
            .contains_key(&(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone()
            )));
    }

    #[test]
    fn test_internal_remove_lease_success_different_owners() {
        let mut contract = Contract::new(accounts(1).into());
        let owner_1: AccountId = accounts(2).into();
        let owner_2: AccountId = accounts(4).into();

        let mut lease_condition_1 = create_lease_condition_default();
        lease_condition_1.state = LeaseState::Active;
        lease_condition_1.token_id = "test_token_1".to_string();
        lease_condition_1.lender_id = owner_1.clone();

        let key_1 = "test_key_1".to_string();
        contract.internal_insert_lease(&key_1, &lease_condition_1);

        let mut lease_condition_2 = create_lease_condition_default();
        lease_condition_2.state = LeaseState::Active;
        lease_condition_2.token_id = "test_token_2".to_string();
        lease_condition_2.lender_id = owner_2.clone();

        let key_2 = "test_key_2".to_string();
        contract.internal_insert_lease(&key_2, &lease_condition_2);

        assert!(contract.lease_map.len() == 2);
        assert!(contract.lease_ids_by_lender.contains_key(&owner_1));
        assert!(contract.lease_ids_by_lender.contains_key(&owner_2));

        contract.internal_remove_lease(&key_1);

        assert!(contract.lease_map.len() == 1);
        assert!(!contract.lease_ids_by_lender.contains_key(&owner_1));
        assert!(contract.lease_ids_by_lender.contains_key(&owner_2));
    }

    #[test]
    fn test_internal_remove_lease_success_different_borrowers() {
        let mut contract = Contract::new(accounts(1).into());
        let borrower_1: AccountId = accounts(3).into();
        let borrower_2: AccountId = accounts(4).into();

        let mut lease_condition_1 = create_lease_condition_default();
        lease_condition_1.state = LeaseState::Active;
        lease_condition_1.token_id = "test_token_1".to_string();
        lease_condition_1.borrower_id = borrower_1.clone();

        let key_1 = "test_key_1".to_string();
        contract.internal_insert_lease(&key_1, &lease_condition_1);

        let mut lease_condition_2 = create_lease_condition_default();
        lease_condition_2.state = LeaseState::Active;
        lease_condition_2.token_id = "test_token_2".to_string();
        lease_condition_2.borrower_id = borrower_2.clone();

        let key_2 = "test_key_2".to_string();
        contract.internal_insert_lease(&key_2, &lease_condition_2);

        assert!(contract.lease_map.len() == 2);
        assert!(contract.lease_ids_by_borrower.contains_key(&borrower_1));
        assert!(contract.lease_ids_by_borrower.contains_key(&borrower_2));

        contract.internal_remove_lease(&key_1);

        assert!(contract.lease_map.len() == 1);
        assert!(!contract.lease_ids_by_borrower.contains_key(&borrower_1));
        assert!(contract.lease_ids_by_borrower.contains_key(&borrower_2));
    }

    // Helper function to return a lease condition using default seting
    fn create_lease_condition_default() -> LeaseCondition {
        let token_id: TokenId = "test_token".to_string();
        let approval_id = 1;
        let lender: AccountId = accounts(2).into();
        let borrower: AccountId = accounts(3).into();
        let nft_address: AccountId = accounts(4).into();
        let ft_contract_addr: AccountId = accounts(5).into();
        let expiration = 1000;
        let price = 5;

        create_lease_condition(
            nft_address,
            token_id.clone(),
            lender.clone(),
            borrower.clone(),
            ft_contract_addr.clone(),
            approval_id,
            expiration.clone(),
            price,
            None,
            LeaseState::Pending,
        )
    }

    // Helper function create a lease condition based on input
    fn create_lease_condition(
        contract_addr: AccountId,
        token_id: TokenId,
        lender_id: AccountId,
        borrower_id: AccountId,
        ft_contract_addr: AccountId,
        approval_id: u64,
        expiration: u64,
        price: u128,
        payout: Option<Payout>,
        state: LeaseState,
    ) -> LeaseCondition {
        LeaseCondition {
            contract_addr,
            token_id,
            lender_id,
            borrower_id,
            ft_contract_addr,
            approval_id,
            expiration,
            price,
            payout,
            state,
        }
    }
}
