use std::collections::HashMap;

use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    bs58, ext_contract, is_promise_success, promise_result_as_success, require, serde_json,
    serde_json::json, CryptoHash, PromiseOrValue,
};
use near_sdk::{env, near_bindgen, AccountId, BorshStorageKey, Gas, PanicOnDefault, Promise};

mod externals;
mod nft;
mod utils;
use crate::externals::*;

// Copied from Paras market contract. Will need to be fine-tuned.
// https://github.com/ParasHQ/paras-marketplace-contract/blob/2dcb9e8b3bc8b9d4135d0f96f0255cd53116a6b4/paras-marketplace-contract/src/lib.rs#L17
pub const TGAS: u64 = 1_000_000_000_000;
pub const XCC_GAS: Gas = Gas(5 * TGAS); // cross contract gas
pub const GAS_FOR_NFT_TRANSFER: Gas = Gas(5 * TGAS);
pub const BASE_GAS: Gas = Gas(5 * TGAS);
pub const GAS_FOR_ROYALTIES: Gas = BASE_GAS;
pub const GAS_FOR_RESOLVE_CLAIM_BACK: Gas = Gas(BASE_GAS.0 * 10u64);
// the tolerance of lease price minus the sum of payout
// Set it to 1 to avoid linter error
pub const PAYOUT_DIFF_TORLANCE_YACTO: u128 = 1;
pub const MAX_LEN_PAYOUT: u32 = 50;

pub type LeaseId = String;
pub type ListingId = String; // marketplace listing_id
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
    PendingOnRent,
    Active,
}
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseJson {
    nft_contract_id: AccountId,
    nft_token_id: TokenId,
    lender_id: AccountId,
    borrower_id: AccountId,
    ft_contract_addr: AccountId,
    price: U128,
    start_ts_nano: u64,
    end_ts_nano: u64,
    nft_payout: Payout,
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
    pub start_ts_nano: u64, // The timestamp in nano to start the lease, i.e. the current user will be the borrower
    pub end_ts_nano: u64, // The timestamp in nano to end the lease, i.e. the lender can claim back the NFT
    pub price: U128,      // Proposed lease price
    pub payout: Option<Payout>, // Payout info (e.g. for Royalty split)
    pub state: LeaseState, // Current lease state
}

#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ContractV1 {
    owner: AccountId,
    lease_map: UnorderedMap<LeaseId, LeaseCondition>,
    lease_ids_by_lender: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_ids_by_borrower: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_id_by_contract_addr_and_token_id: LookupMap<(AccountId, TokenId), LeaseId>,
    active_lease_ids: UnorderedSet<LeaseId>, // This also records all existing LEASE token ids
    active_lease_ids_by_lender: LookupMap<AccountId, UnorderedSet<LeaseId>>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    lease_map: UnorderedMap<LeaseId, LeaseCondition>,
    lease_ids_by_lender: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_ids_by_borrower: LookupMap<AccountId, UnorderedSet<LeaseId>>,
    lease_id_by_contract_addr_and_token_id: LookupMap<(AccountId, TokenId), LeaseId>, // <(NFT_contract, token_id), lease_id>

    active_lease_ids: UnorderedSet<LeaseId>, // This also records all existing LEASE token ids
    active_lease_ids_by_lender: LookupMap<AccountId, UnorderedSet<LeaseId>>,

    // Allowlist of the contract addresses of the FT for the rent payment currency.
    // It's ok to load all allowed FT addresses into memory at once, since it's won't be long.
    allowed_ft_contract_addrs: Vec<AccountId>,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    LendingsKey,
    LeaseIdsByLender,
    LeasesIdsByLenderInner { account_id_hash: CryptoHash },
    LeaseIdsByBorrower,
    LeaseIdsByBorrowerInner { account_id_hash: CryptoHash },
    LeaseIdByContractAddrAndTokenId,
    ActiveLeaseIdsByOwner,
    ActiveLeaseIdsByOwnerInner { account_id_hash: CryptoHash },
    ActiveLeaseIds,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct RentAcceptanceJson {
    nft_contract_id: AccountId,
    nft_token_id: TokenId,
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
            active_lease_ids_by_lender: LookupMap::new(StorageKey::ActiveLeaseIdsByOwner),
            active_lease_ids: UnorderedSet::new(StorageKey::ActiveLeaseIds),
            allowed_ft_contract_addrs: Vec::new(),
        }
    }

    /// A temporary method to completely reset the contract state.
    /// It's the last resort to recover when the contract state got corrupted.
    /// Inspired by: https://gist.github.com/ilyar/19bdc04d1aa09ae0fc84eb4297df1a1d
    #[private]
    #[init(ignore_state)]
    pub fn clean(owner_id: AccountId, keys: Vec<Base64VecU8>) -> Self {
        for key in keys.iter() {
            env::storage_remove(&key.0);
        }
        Self::new(owner_id)
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

        Self::new(prev.owner)
    }

    fn activate_lease(&mut self, lease_id: LeaseId) {
        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();
        let new_lease_condition = LeaseCondition {
            state: LeaseState::Active,
            ..lease_condition
        };
        self.lease_map.insert(&lease_id, &new_lease_condition);

        env::log_str(
            &json!({
                "type": "[INFO] NiFTyRent Rental: A lease has been activated",
                "params": {
                    "lease_id": lease_id.clone(),
                    "lease_state": new_lease_condition.state,
                    "nft_contract": new_lease_condition.contract_addr.clone(),
                    "nft_token_id": new_lease_condition.token_id.clone(),
                }
            })
            .to_string(),
        );

        self.nft_mint(lease_id, new_lease_condition.lender_id.clone());
    }

    #[payable]
    pub fn claim_back(&mut self, lease_id: LeaseId) {
        // Function to allow a user to claim back the NFT and rent after a lease expired.

        let lease_condition: LeaseCondition = self.lease_map.get(&lease_id).unwrap();

        // 1. check expire time
        assert!(
            lease_condition.end_ts_nano < env::block_timestamp(),
            "Lease has not expired yet!"
        );
        // 2. check state == active
        assert_eq!(
            lease_condition.state,
            LeaseState::Active,
            "Queried Lease is not active!"
        );

        // 3. only the current lease lender or service contract owner can claim back from expried lease
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
                    self.internal_transfer_ft(
                        lease_condition.ft_contract_addr.clone(),
                        receiver_id,
                        amount,
                    );
                }
            }
            None => {
                self.internal_transfer_ft(
                    lease_condition.ft_contract_addr.clone(),
                    lease_condition.lender_id,
                    U128::from(lease_condition.price),
                );
            }
        }

        self.internal_remove_lease(&lease_id);
    }

    // private function to transfer FT to receiver_id
    fn internal_transfer_ft(
        &self,
        ft_contract_addr: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> Promise {
        ext_ft_core::ext(ft_contract_addr)
            .with_static_gas(Gas(10 * TGAS))
            .with_attached_deposit(1)
            .ft_transfer(receiver_id, amount, None)
            .as_return()
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

    pub fn lease_by_contract_and_token(
        &self,
        contract_id: AccountId,
        token_id: TokenId,
    ) -> Option<(String, LeaseCondition)> {
        let lease_id = self
            .lease_id_by_contract_addr_and_token_id
            .get(&(contract_id, token_id));

        if lease_id.is_none() {
            return None;
        } else {
            let lease_condition = self.lease_map.get(&lease_id.clone().unwrap()).unwrap();
            return Some((lease_id.unwrap(), lease_condition));
        }
    }

    pub fn active_leases_by_lender(&self, account_id: AccountId) -> Vec<(String, LeaseCondition)> {
        let mut results = vec![];

        let active_lease_ids = self
            .active_lease_ids_by_lender
            .get(&account_id)
            .unwrap_or(UnorderedSet::new(b"s"));
        for id in active_lease_ids.iter() {
            let lease_condition = self.lease_map.get(&id).unwrap();
            results.push((id, lease_condition));
        }
        return results;
    }

    #[private]
    pub fn get_lease_by_contract_and_token(
        &self,
        contract_id: AccountId,
        token_id: TokenId,
    ) -> Option<LeaseCondition> {
        let lease_id = self
            .lease_id_by_contract_addr_and_token_id
            .get(&(contract_id, token_id));

        if lease_id.is_none() {
            return None;
        } else {
            return Some(self.lease_map.get(&lease_id.unwrap()).unwrap());
        }
    }

    pub fn get_borrower_by_contract_and_token(
        &self,
        contract_id: AccountId,
        token_id: TokenId,
    ) -> Option<AccountId> {
        // return the current borrower of the NFTs
        // Only active lease has valid borrower

        let lease_condition_option = self.get_lease_by_contract_and_token(contract_id, token_id);
        if lease_condition_option.is_none() {
            return None;
        }

        let lease_condition = lease_condition_option.unwrap();

        if lease_condition.state == LeaseState::Active {
            // only active lease has valid borrower
            return Some(lease_condition.borrower_id);
        } else {
            return None;
        }
    }

    pub fn get_current_user_by_contract_and_token(
        &self,
        contract_id: AccountId,
        token_id: TokenId,
    ) -> Option<AccountId> {
        // return the current user of the NFTs
        // The current user of an active lease is the borrower, otherwise it is the lender

        let lease_condition_option = self.get_lease_by_contract_and_token(contract_id, token_id);

        assert!(
            !lease_condition_option.is_none(),
            "Cannot find a lease of this contract and token!"
        );

        let lease_condition = lease_condition_option.unwrap();
        if lease_condition.state == LeaseState::Active
            && lease_condition.start_ts_nano < env::block_timestamp()
            && lease_condition.end_ts_nano > env::block_timestamp()
        {
            return Some(lease_condition.borrower_id);
        } else {
            return Some(lease_condition.lender_id);
        }
    }

    pub fn set_allowed_ft_contract_addrs(&mut self, addrs: Vec<AccountId>) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only the owner can set allowed FT contracts"
        );

        self.allowed_ft_contract_addrs = addrs
    }

    pub fn get_allowed_ft_contract_addrs(&self) -> Vec<AccountId> {
        self.allowed_ft_contract_addrs.clone()
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
        nft_contract_id: AccountId,
        nft_token_id: TokenId,
        owner_id: AccountId,
        borrower_id: AccountId,
        ft_contract_addr: AccountId,
        start_ts_nano: u64,
        end_ts_nano: u64,
        price: U128,
        nft_payout: Payout,
    ) -> bool {
        // TODO(syu): log can be removed
        env::log_str(
            &json!({
                "type": "[DEBUG] NiFTyRent Rental: processing payput for nft token.",
                "params": {
                    "nft_contract_id": nft_contract_id.clone(),
                    "nft_token_id": nft_token_id.clone(),
                    "lender": owner_id.clone(),
                    "borrower": borrower_id.clone(),
                    "nft_payout": nft_payout.clone(),
                }
            })
            .to_string(),
        );

        // build lease condition from the parsed json
        let lease_condition: LeaseCondition = LeaseCondition {
            contract_addr: nft_contract_id,
            token_id: nft_token_id,
            lender_id: owner_id.clone(),
            borrower_id: borrower_id,
            ft_contract_addr: ft_contract_addr,
            price: price,
            start_ts_nano: start_ts_nano,
            end_ts_nano: end_ts_nano,
            payout: Some(nft_payout),
            state: LeaseState::PendingOnRent,
        };

        let seed = near_sdk::env::random_seed();
        let lease_id = bs58::encode(seed)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string();

        self.internal_insert_lease(&lease_id, &lease_condition);

        // return false to indict no need to revert the nft transfer
        return false;
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

        // Clean up NFT related fields
        // update active leases set
        self.active_lease_ids.remove(&lease_id);

        // update active_lease_ids_by_lender
        let mut active_lease_id_set = self
            .active_lease_ids_by_lender
            .get(&lease_condition.lender_id);

        if let Some(active_lease_id_set) = active_lease_id_set.as_mut() {
            active_lease_id_set.remove(&lease_id);

            if active_lease_id_set.is_empty() {
                self.active_lease_ids_by_lender
                    .remove(&lease_condition.lender_id);
            } else {
                self.active_lease_ids_by_lender
                    .insert(&lease_condition.lender_id, &active_lease_id_set);
            }
        }
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

        // log lease insertion
        env::log_str(
            &json!({
                "type": "[INFO] NiFTyRent Rental: A new lease has been inserted.",
                "params": {
                    "lease_id": lease_id.clone(),
                    "nft_contract_id": lease_condition.contract_addr.clone(),
                    "nft_token_id": lease_condition.token_id.clone(),
                    "lender": lease_condition.lender_id.clone(),
                    "borrower": lease_condition.borrower_id.clone(),
                    "lease_state": lease_condition.state,
                }
            })
            .to_string(),
        );
    }

    /// This function updates only the lender info in an active lease
    /// All affecting indices will be updated
    fn internal_update_active_lease_lender(
        &mut self,
        old_lender: &AccountId,
        new_lender: &AccountId,
        lease_id: &LeaseId,
    ) {
        // 1. Check if the active lease exist
        assert_eq!(
            self.active_lease_ids.contains(lease_id),
            true,
            "Only active lease can update lender!"
        );

        // 2. Ensure the given active lease belongs to the old owner
        let mut active_lease_ids_set = self
            .active_lease_ids_by_lender
            .get(old_lender)
            .expect("Active Lease is not owned by the old lender!");

        // 3. Remove the active lease from the old lender
        // update index for active lease ids
        active_lease_ids_set.remove(lease_id);
        if active_lease_ids_set.is_empty() {
            self.active_lease_ids_by_lender.remove(old_lender);
        } else {
            self.active_lease_ids_by_lender
                .insert(old_lender, &active_lease_ids_set);
        }
        // Update the index for lease ids by lender for old lender
        let mut lease_ids_set = self.lease_ids_by_lender.get(old_lender).unwrap();
        lease_ids_set.remove(lease_id);
        if lease_ids_set.is_empty() {
            self.lease_ids_by_lender.remove(old_lender);
        } else {
            self.lease_ids_by_lender.insert(old_lender, &lease_ids_set);
        }

        // 4. Add the active lease to the new lender
        // update the index for active lease ids
        let mut active_lease_ids_set = self
            .active_lease_ids_by_lender
            .get(new_lender)
            .unwrap_or_else(|| {
                // if the new lender doesn't have any active lease, create a new record
                UnorderedSet::new(
                    StorageKey::ActiveLeaseIdsByOwnerInner {
                        account_id_hash: utils::hash_account_id(new_lender),
                    }
                    .try_to_vec()
                    .unwrap(),
                )
            });
        active_lease_ids_set.insert(lease_id);
        self.active_lease_ids_by_lender
            .insert(new_lender, &active_lease_ids_set);
        // Udpate the index for lease ids by lender for new lender
        let mut lease_ids_set = self.lease_ids_by_lender.get(new_lender).unwrap_or_else(|| {
            // if the receiver doesn;t have any lease, create a new record
            UnorderedSet::new(
                StorageKey::LeasesIdsByLenderInner {
                    account_id_hash: utils::hash_account_id(new_lender),
                }
                .try_to_vec()
                .unwrap(),
            )
        });
        lease_ids_set.insert(lease_id);
        self.lease_ids_by_lender.insert(new_lender, &lease_ids_set);

        // 5. Update the lease map index accordingly
        let mut lease_condition = self.lease_map.get(lease_id).unwrap();
        lease_condition.lender_id = new_lender.clone();
        self.lease_map.insert(&lease_id, &lease_condition); // insert data back to persis the value
    }
}

/**
 * Trait that will handle the receival of the leasing NFT.
 * When the Marketplace calls nft_transfer_call on NFT contract, the NFT contract
 * will invoke this function.
*/
trait NonFungibleTokenTransferReceiver {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool>;
}

#[near_bindgen]
impl NonFungibleTokenTransferReceiver for Contract {
    /**
     * 1. Check NFT transfer is successful
     * 2. Create proxy payouts if not supported
     * 3. Create a lease
     */
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        // Enforce cross contract call
        let nft_contract_id = env::predecessor_account_id();
        assert_ne!(
            env::current_account_id(),
            nft_contract_id,
            "nft_on_transfer should only be called via XCC."
        );

        let lease_json: LeaseJson =
            near_sdk::serde_json::from_str(&msg).expect("Invalid lease json!");

        // Enforce the leasing token is the same as the transferring token
        assert_eq!(nft_contract_id, lease_json.nft_contract_id);
        assert_eq!(token_id, lease_json.nft_token_id);

        // log nft transfer
        env::log_str(
            &json!({
                "type": "[DEBUG] NiFTyRent Rental: Checking payout for leasing NFT.",
                "params": {
                    "nft_contract_id": nft_contract_id.clone(),
                    "nft_token_id": token_id.clone(),
                    "nft_payout": lease_json.nft_payout.clone(),
                }
            })
            .to_string(),
        );

        // Create a lease after resolving payouts of the leasing token
        ext_self::ext(env::current_account_id())
            .with_static_gas(GAS_FOR_ROYALTIES)
            .create_lease_with_payout(
                lease_json.nft_contract_id,
                lease_json.nft_token_id,
                lease_json.lender_id, // use lender here, as the token owner has been updated to Rental contract
                lease_json.borrower_id,
                lease_json.ft_contract_addr,
                lease_json.start_ts_nano,
                lease_json.end_ts_nano,
                lease_json.price,
                lease_json.nft_payout.clone()
            )
            .into()

        // Create a lease after resolving payouts of the leasing token
        // ext_nft::ext(lease_json.nft_contract_id.clone())
        //     .nft_payout(
        //         lease_json.nft_token_id.clone(), // token_id
        //         U128::from(lease_json.price.0),  // price
        //         Some(MAX_LEN_PAYOUT),            // max_len_payout
        //     )
        //     .then(
        //         ext_self::ext(env::current_account_id())
        //             .with_static_gas(GAS_FOR_ROYALTIES)
        //             .create_lease_with_payout(
        //                 lease_json.nft_contract_id,
        //                 lease_json.nft_token_id,
        //                 lease_json.lender_id,  // use lender here, as the token owner has been updated to Rental contract
        //                 lease_json.borrower_id,
        //                 lease_json.ft_contract_addr,
        //                 lease_json.start_ts_nano,
        //                 lease_json.end_ts_nano,
        //                 lease_json.price,
        //             ),
        //     )
        //     .into()
    }
}

/*
    The trait for receiving rent transfer from marketplace.
    Depending on the FT contract implementation, it may need the users to register to deposit.
    So far we do not check if all partis have registered thier account on the FT contract,
        - Lender: he should make sure he has registered otherwise he will not receive the payment
        - Borrower: he cannot accept the lease if he does not register
        - Royalty payments: if any accounts in the royalty didn't register, they will not receive the payout. That
                             part of payment will be kept in this smart contract
*/
#[ext_contract(ext_ft_receiver)]
pub trait FungibleTokenReceiver {
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128;
}

/**
 * This method receives borrower's rent transferred by markeplace. It also trigers a lease activation
 * 1. Marketplace(Sender) calls `ft_transfer_call` on FT contract.
 * 2. FT contract transfers `amount` tokens from marketplace to core rental contract (reciever).
 * 3. FT contract calls `ft_on_transfer` on core rental contract.
 * 4. Rental contract updates lease state accordingly. Rent condition checks have been performed on marketplace side.
 * 5. Rental contract returns Promise accordingly.
 */
#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    #[payable]
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        // update the lease state to from PendingOnRent to active

        // Enforce cross contract call
        let ft_contract_id = env::predecessor_account_id();
        assert_ne!(
            env::current_account_id(),
            ft_contract_id,
            "ft_on_transfer should only be called via XCC."
        );

        // Extract recived message
        let rent_acceptance_json: RentAcceptanceJson =
            near_sdk::serde_json::from_str(&msg).expect("Not valid listing id data!");

        // Find the targeting lease
        let lease_condition = self
            .get_lease_by_contract_and_token(
                rent_acceptance_json.nft_contract_id.clone(),
                rent_acceptance_json.nft_token_id.clone(),
            )
            .expect("The targeting lease does not exist!");

        // Enforce the ft contract matches
        assert_eq!(
            ft_contract_id, lease_condition.ft_contract_addr,
            "Wrong FT contract address!"
        );

        // Enforce the rent amount matches
        assert_eq!(
            amount.0, lease_condition.price.0,
            "Transferred amount doesn't match the asked rent!"
        );

        // Update the lease state accordingly
        assert_eq!(
            lease_condition.state,
            LeaseState::PendingOnRent,
            "This lease is not pending on rent!"
        );

        let lease_id = self
            .lease_id_by_contract_addr_and_token_id
            .get(&(
                rent_acceptance_json.nft_contract_id,
                rent_acceptance_json.nft_token_id,
            ))
            .expect("The targeting lease id does not exist!");

        self.activate_lease(lease_id);

        // Specify the unused amount as required by NEP-141
        let unused_ammount: U128 = U128::from(0);
        return unused_ammount;
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
    use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig, VMConfig};

    #[test]
    fn test_new() {
        let contract = Contract::new(accounts(1).into());
        assert_eq!(accounts(1), contract.owner);
        assert!(UnorderedMap::is_empty(&contract.lease_map));
    }

    // TODO(syu): borrower check is done on marketside. Maybe update to check target lease exist, or remove this
    fn test_lending_accept_wrong_borrower() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let lease_id = "test_key".to_string();

        contract.lease_map.insert(&lease_id, &lease_condition);
        let wrong_borrower: AccountId = accounts(4).into();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(lease_condition.ft_contract_addr.clone())
            .build());

        contract.ft_on_transfer(
            wrong_borrower.clone(),
            U128::from(lease_condition.price),
            json!({ "lease_id": lease_id }).to_string(),
        );
    }

    #[test]
    #[should_panic(expected = "Wrong FT contract address!")]
    fn test_lending_accept_fail_wrong_ft_addr() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let lease_id = "test_lease_id".to_string();
        let wrong_ft_addr = accounts(0);
        contract.lease_map.insert(&lease_id, &lease_condition);
        // needed for finding the target lease_condition at ft_on_transfer
        contract.lease_id_by_contract_addr_and_token_id.insert(
            &(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone(),
            ),
            &lease_id,
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(wrong_ft_addr.into())
            .build());

        let msg_rent_transfer_json = json!({
            "nft_contract_id": lease_condition.contract_addr.clone().to_string(),
            "nft_token_id": lease_condition.token_id.clone().to_string(),
        })
        .to_string();

        contract.ft_on_transfer(
            lease_condition.borrower_id.clone(),
            U128::from(lease_condition.price),
            msg_rent_transfer_json,
        );
    }

    #[test]
    #[should_panic(expected = "Transferred amount doesn't match the asked rent!")]
    fn test_lending_accept_fail_wrong_rent() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let lease_id = "test_lease_id".to_string();
        contract.lease_map.insert(&lease_id, &lease_condition);
        // needed for finding the target lease_condition at ft_on_transfer
        contract.lease_id_by_contract_addr_and_token_id.insert(
            &(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone(),
            ),
            &lease_id,
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(lease_condition.ft_contract_addr.clone())
            .build());

        let msg_rent_transfer_json = json!({
            "nft_contract_id": lease_condition.contract_addr.clone().to_string(),
            "nft_token_id": lease_condition.token_id.clone().to_string(),
        })
        .to_string();

        contract.ft_on_transfer(
            lease_condition.borrower_id.clone(),
            U128::from(lease_condition.price.0 - 1),
            msg_rent_transfer_json,
        );
    }

    #[test]
    #[should_panic(expected = "This lease is not pending on rent!")]
    fn test_lending_accept_fail_wrong_lease_state() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        let lease_id = "test_lease_id".to_string();
        contract.lease_map.insert(&lease_id, &lease_condition);
        // needed for finding the target lease_condition at ft_on_transfer
        contract.lease_id_by_contract_addr_and_token_id.insert(
            &(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone(),
            ),
            &lease_id,
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(lease_condition.ft_contract_addr.clone())
            .build());

        let msg_rent_transfer_json = json!({
            "nft_contract_id": lease_condition.contract_addr.clone().to_string(),
            "nft_token_id": lease_condition.token_id.clone().to_string(),
        })
        .to_string();

        contract.ft_on_transfer(
            lease_condition.borrower_id.clone(),
            U128::from(lease_condition.price),
            msg_rent_transfer_json,
        );
    }

    #[test]
    fn test_lending_accept_success() {
        let mut contract = Contract::new(accounts(1).into());
        let lease_condition = create_lease_condition_default();
        let lease_id = "test_lease_id".to_string();
        contract.lease_map.insert(&lease_id, &lease_condition);
        // needed for finding the target lease_condition at ft_on_transfer
        contract.lease_id_by_contract_addr_and_token_id.insert(
            &(
                lease_condition.contract_addr.clone(),
                lease_condition.token_id.clone(),
            ),
            &lease_id,
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(lease_condition.ft_contract_addr.clone())
            .build());

        let msg_rent_transfer_json = json!({
            "nft_contract_id": lease_condition.contract_addr.clone().to_string(),
            "nft_token_id": lease_condition.token_id.clone().to_string(),
        })
        .to_string();

        contract.ft_on_transfer(
            lease_condition.borrower_id.clone(),
            U128::from(lease_condition.price),
            msg_rent_transfer_json,
        );

        // Nothing can be checked, except the fact the call doesn't panic.
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
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Failed],
        );

        contract.activate_lease(key.clone());

        let lease_condition_result = contract.lease_map.get(&key).unwrap();
        assert_eq!(lease_condition_result.payout, None);
        assert_eq!(lease_condition_result.state, LeaseState::PendingOnRent);
    }

    #[test]
    #[should_panic(expected = "Lease has not expired yet!")]
    fn test_claim_back_not_expired_yet() {
        let mut contract = Contract::new(accounts(1).into());

        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.end_ts_nano = 1000;

        let key = "test_key".to_string();
        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.end_ts_nano - 1)
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
            .block_timestamp(lease_condition.end_ts_nano + 1)
            .build());

        contract.claim_back(key);
    }

    #[test]
    #[should_panic(expected = "Queried Lease is not active!")]
    fn test_claim_back_inactive_lease() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::PendingOnRent;
        let key = "test_key".to_string();

        contract.lease_map.insert(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.end_ts_nano + 1)
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
            .block_timestamp(lease_condition.end_ts_nano + 1)
            .build());

        let non_existing_key = "dummy_key".to_string();
        contract.claim_back(non_existing_key);
    }

    #[test]
    fn test_claim_back_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.price = U128::from(20);
        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(lease_condition.lender_id.clone())
            .block_timestamp(lease_condition.end_ts_nano + 1)
            .build());

        contract.claim_back(key);

        // Nothing can be checked, except the fact the call doesn't panic.
    }

    #[test]
    fn test_create_lease_with_payout_succeeds_when_nft_payout_xcc_succeeded() {
        let mut contract = Contract::new(accounts(1).into());
        let nft_contract_id: AccountId = accounts(4).into();
        let token_id: TokenId = "test_token".to_string();
        let owner_id: AccountId = accounts(2).into();
        let borrower_id: AccountId = accounts(3).into();
        let ft_contract_addr: AccountId = accounts(4).into();
        let price: U128 = U128::from(5);

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
            0,
            1000,
            price,
        );

        assert!(!contract.lease_map.is_empty());
        let lease_condition = &contract.leases_by_owner(owner_id.clone())[0].1;

        assert_eq!(nft_contract_id, lease_condition.contract_addr);
        assert_eq!(token_id, lease_condition.token_id);
        assert_eq!(owner_id, lease_condition.lender_id);
        assert_eq!(borrower_id, lease_condition.borrower_id);
        assert_eq!(5, lease_condition.price.0);
        assert_eq!(1000, lease_condition.end_ts_nano);
        assert_eq!(Some(payout), lease_condition.payout);
    }

    #[test]
    fn test_create_lease_with_payout_succeeds_when_nft_payout_xcc_failed() {
        // When nft_payout xcc failed, we should still produce a internal payout record,
        // allocating to the original lender the whole price.

        let mut contract = Contract::new(accounts(1).into());

        let nft_contract_id: AccountId = accounts(1).into();
        let token_id: TokenId = "test_token".to_string();
        let owner_id: AccountId = accounts(2).into();
        let borrower_id: AccountId = accounts(3).into();
        let ft_contract_addr: AccountId = accounts(1).into();
        let price: U128 = U128::from(5);

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(borrower_id.clone())
                .attached_deposit(price.0)
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Failed],
        );

        contract.create_lease_with_payout(
            nft_contract_id.clone(),
            token_id.clone(),
            owner_id.clone(),
            borrower_id.clone(),
            ft_contract_addr,
            0,
            1000,
            price,
        );

        let payout_expected = Payout {
            payout: HashMap::from([(owner_id.clone().into(), U128::from(price.clone()))]),
        };
        let lease_condition = &contract.leases_by_owner(owner_id.clone())[0].1;

        assert!(lease_condition.payout.is_some());
        assert_eq!(
            1,
            lease_condition.payout.as_ref().unwrap().payout.keys().len()
        );
        assert!(lease_condition
            .payout
            .as_ref()
            .unwrap()
            .payout
            .contains_key(&owner_id));
        assert_eq!(Some(payout_expected), lease_condition.payout);
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
        let price: U128 = U128::from(5);

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
            0,
            1000,
            price,
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

        lease_condition.state = LeaseState::PendingOnRent;
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
    fn test_get_current_user_by_contract_and_token_success_found_matching_borrower() {
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

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .block_timestamp(10)
            .build());

        let result_owner = contract
            .get_current_user_by_contract_and_token(expected_contract_address, expected_token_id)
            .unwrap();
        assert!(result_owner == expected_borrower_id);
    }

    #[test]
    fn test_get_current_user_by_contract_and_token_success_lease_is_inactive() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_lender_id: AccountId = accounts(2).into();
        let expected_borrower_id: AccountId = accounts(3).into();

        lease_condition.state = LeaseState::PendingOnRent;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.lender_id = expected_lender_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        let result_owner = contract
            .get_current_user_by_contract_and_token(expected_contract_address, expected_token_id)
            .unwrap();
        assert!(result_owner == expected_lender_id);
    }

    #[test]
    fn test_get_current_user_by_contract_and_token_success_after_lease_expires() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_borrower_id: AccountId = accounts(3).into();
        let expected_lender_id: AccountId = accounts(2).into();

        lease_condition.state = LeaseState::Active;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.lender_id = expected_lender_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .block_timestamp(1000)
            .build());

        let result_owner = contract
            .get_current_user_by_contract_and_token(expected_contract_address, expected_token_id)
            .unwrap();
        assert!(result_owner == expected_lender_id);
    }

    #[test]
    fn test_get_current_user_by_contract_and_token_success_lease_not_start() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();

        let expected_contract_address: AccountId = accounts(4).into();
        let expected_token_id = "test_token".to_string();
        let expected_lender_id: AccountId = accounts(2).into();
        let expected_borrower_id: AccountId = accounts(3).into();

        lease_condition.state = LeaseState::PendingOnRent;
        lease_condition.contract_addr = expected_contract_address.clone();
        lease_condition.token_id = expected_token_id.clone();
        lease_condition.lender_id = expected_lender_id.clone();
        lease_condition.borrower_id = expected_borrower_id.clone();
        // 2333/01/01 00:00
        lease_condition.start_ts_nano = 11455171200000000000;

        let key = "test_key".to_string();
        contract.internal_insert_lease(&key, &lease_condition);

        let result_owner = contract
            .get_current_user_by_contract_and_token(expected_contract_address, expected_token_id)
            .unwrap();
        assert!(result_owner == expected_lender_id);
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

    /// Creat two leases using the same lender
    /// Before the leases got actived, active_leases_by_lender() should return 0 leases
    /// After both leases got actived, active_leases_by_lender() should return 2 leases
    #[test]
    fn test_active_leases_by_lender_succeeds() {
        let mut contract = Contract::new(accounts(0).into());
        let expected_lender_id: AccountId = accounts(2).into();

        let mut lease_condition_1 = create_lease_condition_default();
        lease_condition_1.token_id = "test_token_1".to_string();
        lease_condition_1.lender_id = expected_lender_id.clone();
        let key_1 = "test_key_1".to_string();
        contract.internal_insert_lease(&key_1, &lease_condition_1);

        let mut lease_condition_2 = create_lease_condition_default();
        lease_condition_2.token_id = "test_token_2".to_string();
        lease_condition_2.lender_id = expected_lender_id.clone();
        let key_2 = "test_key_2".to_string();
        contract.internal_insert_lease(&key_2, &lease_condition_2);

        // check before the leases got activated
        let active_leases = contract.active_leases_by_lender(expected_lender_id.clone());
        assert_eq!(active_leases.len(), 0);

        // activate the 1st lease
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(Vec::new())],
        );
        contract.activate_lease(key_1.clone());

        // activate the 2nd lease
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(Vec::new())],
        );
        contract.activate_lease(key_2.clone());

        // test after the leases got activated
        let active_leases = contract.active_leases_by_lender(expected_lender_id.clone());
        assert_eq!(active_leases.len(), 2);
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
            .block_timestamp(lease_condition_1.end_ts_nano + 1)
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
            .block_timestamp(lease_condition_1.end_ts_nano + 1)
            .build());

        let result = contract.leases_by_owner(expected_owner_id.clone());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_internal_insert_lease_success() {
        let mut contract = Contract::new(accounts(1).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::Active;
        lease_condition.price = U128::from(20);
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
        lease_condition.price = U128::from(20);
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

    /// 1. Initially, Alice owns an active lease
    /// 2. Alice transfers the lease to Bob
    /// 3. Check Success:
    ///    - Lease record for Alice is emptied
    ///    - Lease record for Bob is updated
    #[test]
    fn test_internal_update_active_lease_lender_succeeds() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.lender_id = accounts(0).into(); //Alice

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);

        // update active lease records for Alice
        lease_condition.state = LeaseState::Active;
        contract.nft_mint(lease_key.clone(), lease_condition.lender_id.clone());

        contract.internal_update_active_lease_lender(
            &lease_condition.lender_id, // Alice
            &accounts(1).into(),        // Bob
            &lease_key,
        );

        assert_eq!(1, contract.active_lease_ids.len());
        assert!(!contract
            .active_lease_ids_by_lender
            .contains_key(&accounts(0).into()));
        assert!(!contract
            .lease_ids_by_lender
            .contains_key(&accounts(0).into()));
        assert!(contract
            .active_lease_ids_by_lender
            .contains_key(&accounts(1).into()));
        assert!(contract
            .lease_ids_by_lender
            .contains_key(&accounts(1).into()));
        assert_eq!(
            contract.lease_map.get(&lease_key).unwrap().lender_id,
            accounts(1).into()
        );
    }

    /// 1. Initially, Alice owns an active lease
    /// 2. Charlie tries to transfer Alice's lease to Bob
    /// 3. Panic, due to unmatching lenders
    #[test]
    #[should_panic(expected = "Active Lease is not owned by the old lender!")]
    fn test_internal_update_active_lease_lender_fails_unmatched_old_lender() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.lender_id = accounts(1).into(); //Alice

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);

        // update active lease records
        lease_condition.state = LeaseState::Active;
        contract.active_lease_ids.insert(&lease_key);
        let mut active_lease_ids_set: UnorderedSet<String> = UnorderedSet::new(
            StorageKey::ActiveLeaseIdsByOwnerInner {
                account_id_hash: utils::hash_account_id(&lease_condition.lender_id),
            }
            .try_to_vec()
            .unwrap(),
        );
        active_lease_ids_set.insert(&lease_key);

        contract.internal_update_active_lease_lender(
            &accounts(3).into(), // Charlie
            &accounts(2).into(), // Bob
            &lease_key,
        );
    }

    #[test]
    #[should_panic(expected = "Only active lease can update lender!")]
    fn test_internal_update_active_lease_lender_fails_not_an_active_lease() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.state = LeaseState::PendingOnRent;

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);

        contract.internal_update_active_lease_lender(
            &lease_condition.lender_id, // Alice
            &accounts(2).into(),        // Bob
            &lease_key,
        );
    }

    #[test]
    #[should_panic(expected = "Only the owner can set allowed FT contracts")]
    fn test_update_allowed_contract_addrs_fail_when_called_by_nonowner() {
        let mut contract = Contract::new(accounts(1).into());
        assert!(contract.get_allowed_ft_contract_addrs().is_empty());

        contract.set_allowed_ft_contract_addrs(vec![accounts(2)]);
    }

    #[test]
    fn test_update_allowed_contract_addrs_success() {
        let mut contract = Contract::new(accounts(1).into());
        assert!(contract.get_allowed_ft_contract_addrs().is_empty());

        testing_env!(VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(1))
            .build());
        contract.set_allowed_ft_contract_addrs(vec![accounts(2), accounts(3)]);
        assert_eq!(
            contract.get_allowed_ft_contract_addrs(),
            vec![accounts(2), accounts(3)]
        );
        contract.set_allowed_ft_contract_addrs(vec![accounts(4)]);
        assert_eq!(contract.get_allowed_ft_contract_addrs(), vec![accounts(4)]);
    }

    // Helper function to return a lease condition using default seting
    pub(crate) fn create_lease_condition_default() -> LeaseCondition {
        let token_id: TokenId = "test_token".to_string();
        let lender: AccountId = accounts(2).into();
        let borrower: AccountId = accounts(3).into();
        let nft_address: AccountId = accounts(4).into();
        let ft_contract_addr: AccountId = accounts(5).into();
        let start_ts_nano = 1;
        let end_ts_nano = 1000;
        let price = U128::from(5);

        create_lease_condition(
            nft_address,
            token_id.clone(),
            lender.clone(),
            borrower.clone(),
            ft_contract_addr.clone(),
            start_ts_nano.clone(),
            end_ts_nano.clone(),
            price,
            None,
            LeaseState::PendingOnRent,
        )
    }

    // helper method to generate a dummy AccountId using input name
    pub(crate) fn create_a_dummy_account_id(account_name: &str) -> AccountId {
        AccountId::new_unchecked(account_name.to_string())
    }

    // Helper function create a lease condition based on input
    fn create_lease_condition(
        contract_addr: AccountId,
        token_id: TokenId,
        lender_id: AccountId,
        borrower_id: AccountId,
        ft_contract_addr: AccountId,
        start_ts_nano: u64,
        end_ts_nano: u64,
        price: U128,
        payout: Option<Payout>,
        state: LeaseState,
    ) -> LeaseCondition {
        LeaseCondition {
            contract_addr,
            token_id,
            lender_id,
            borrower_id,
            ft_contract_addr,
            start_ts_nano,
            end_ts_nano,
            price,
            payout,
            state,
        }
    }
}
