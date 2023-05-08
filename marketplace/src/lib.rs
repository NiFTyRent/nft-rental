use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, UnorderedMap, UnorderedSet},
    env::{self},
    ext_contract, is_promise_success,
    json_types::{U128, U64},
    near_bindgen, promise_result_as_success, require,
    serde::{Deserialize, Serialize},
    serde_json,
    serde_json::json,
    AccountId, BorshStorageKey, CryptoHash, Gas, PanicOnDefault, PromiseResult,
};
use std::collections::HashMap;

mod externals;
mod ft_callbacks;
mod nft_callbacks;
use crate::externals::*;

pub const TGAS: u64 = 1_000_000_000_000;
pub const BASE_GAS: Gas = Gas(5 * TGAS);
pub const GAS_FOR_ROYALTIES: Gas = BASE_GAS;
// the tolerance of lease price minus the sum of payout
// Set it to 1 to avoid linter error
pub const PAYOUT_DIFF_TORLANCE_YACTO: u128 = 1;

// In the current design, one nft token can only have one active lease, even at different rental periods.
// (NFT Contract, NFT Token ID).
type ListingId = (AccountId, TokenId);

// type used for storing nft token's payout
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

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Listing {
    /// The NFT owner
    pub owner_id: AccountId,
    /// The approval id for transfering the NFT into rental contract's custody
    pub approval_id: u64,
    pub nft_contract_id: AccountId,
    pub nft_token_id: TokenId,
    pub ft_contract_id: AccountId,
    pub price: U128,
    pub lease_start_ts_nano: u64,
    pub lease_end_ts_nano: u64,
    /// Lease token's payout info
    pub payout: Option<Payout>,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    /// The admin account
    pub owner_id: AccountId,
    /// The account to receive the marketplace fee. (Currently no fees are collected yet.)
    pub treasury_id: AccountId,
    /// The rental proxy contract (i.e. the core contract) id this marketplace use.
    pub rental_contract_id: AccountId,
    pub listing_by_id: UnorderedMap<ListingId, Listing>,
    /// Whitelist of FT contracts for rent payment.
    pub allowed_ft_contract_ids: UnorderedSet<AccountId>,
    // TODO(libo): Shops?
    pub allowed_nft_contract_ids: UnorderedSet<AccountId>,

    /// Indices of listing for quick lookup.
    pub listing_ids_by_owner_id: LookupMap<AccountId, UnorderedSet<ListingId>>,
    pub listing_ids_by_nft_contract_id: LookupMap<AccountId, UnorderedSet<ListingId>>,
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    Listings,
    FTTokenIds,
    NFTContractIds,
    ListingsByOwnerId,
    ListingsByOwnerIdInner { account_id_hash: CryptoHash },
    ListingsByNftContractId,
    ListingsByNftContractIdInner { account_id_hash: CryptoHash },
}

#[near_bindgen]
impl Contract {
    // ------------------ Initialization -----------------
    #[init]
    pub fn new(owner_id: AccountId, treasury_id: AccountId, rental_contract_id: AccountId) -> Self {
        Self {
            owner_id: owner_id.into(),
            treasury_id: treasury_id.into(),
            rental_contract_id,
            listing_by_id: UnorderedMap::new(StorageKey::Listings),
            allowed_ft_contract_ids: UnorderedSet::new(StorageKey::FTTokenIds),
            allowed_nft_contract_ids: UnorderedSet::new(StorageKey::NFTContractIds),
            listing_ids_by_owner_id: LookupMap::new(StorageKey::ListingsByOwnerId),
            listing_ids_by_nft_contract_id: LookupMap::new(StorageKey::ListingsByNftContractId),
        }
    }

    // ------------------ Admin Functions -----------------

    /// Set the treasury account to keep accured fees in marketplace
    pub fn set_treasury(&mut self, treasury_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.treasury_id = treasury_id;
    }

    #[payable]
    pub fn add_allowed_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        insert_accounts(nft_contract_ids, &mut self.allowed_nft_contract_ids);
    }

    #[payable]
    pub fn remove_allowed_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        remove_accounts(nft_contract_ids, &mut self.allowed_nft_contract_ids);
    }

    #[payable]
    pub fn add_allowed_ft_contract_ids(&mut self, ft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        insert_accounts(ft_contract_ids, &mut self.allowed_ft_contract_ids);
    }

    // ------------------ View Functions -----------------
    /// List all NFT contract that are allowed to be listed in the market.
    pub fn list_allowed_nft_contract_ids(&self) -> Vec<AccountId> {
        return self.allowed_nft_contract_ids.to_vec();
    }
    /// List all FT contract that are allowed to be used for payment.
    pub fn list_allowed_ft_contract_ids(&self) -> Vec<AccountId> {
        return self.allowed_ft_contract_ids.to_vec();
    }

    // TODO(syu): check if the reuturn should be a vector of <(listing_id, listing)>, instead of just listing
    pub fn list_listings_by_owner_id(&self, owner_id: AccountId) -> Vec<Listing> {
        return self
            .listing_ids_by_owner_id
            .get(&owner_id)
            .unwrap_or(UnorderedSet::new(StorageKey::Listings))
            .iter()
            .map(|list_id| self.listing_by_id.get(&list_id).unwrap())
            .collect::<Vec<_>>();
    }

    pub fn list_listings_by_nft_contract_id(&self, nft_contract_id: AccountId) -> Vec<Listing> {
        return self
            .listing_ids_by_nft_contract_id
            .get(&nft_contract_id)
            .unwrap_or(UnorderedSet::new(StorageKey::Listings))
            .iter()
            .map(|list_id| self.listing_by_id.get(&list_id).unwrap())
            .collect::<Vec<_>>();
    }

    pub fn get_listing_by_id(&self, listing_id: ListingId) -> Listing {
        return self
            .listing_by_id
            .get(&listing_id)
            .expect("Listing not found");
    }

    pub fn get_rental_contract_id(&self) -> AccountId {
        return self.rental_contract_id.clone();
    }

    // ------------------ XCC RPCs -----------------
    /**
     * This method will handle the transfer of rent to Core rental contract,
     * depending on the leasing nft transfer result.
     * Rent will only be transfered to Core, if leasing nft has been transferred correctly.
     * Otherwise, no rent transfer.
     * This XCC can only be called by this contract itself. Thus made private.
     */
    #[private]
    pub fn transfer_rent_after_nft_transfer(
        &mut self,
        ft_contract_id: AccountId,
        amount: U128,
        memo: Option<String>,
        listing_id: ListingId,
    ) -> U128 {
        // previoux XCC should be successful
        require!(
            is_promise_success(),
            "NFT transfer failed. Abort rent transfer!"
        );

        // previoux XCC, nft_transfer_call, should not result in reverting the transfer
        // expected status: SuccessValue(`true`)
        if let PromiseResult::Successful(value) = env::promise_result(0) {
            if let Ok(token_transfered) = near_sdk::serde_json::from_slice::<bool>(&value) {
                require!(
                    token_transfered, // true to
                    "NFT transfer wasn't successful. Abort rent transfer!"
                );
            }
        }

        // Trasnfer the rent to Core contract.
        // msg to be passed in ft_transfer_call. Used for specifying the targeting lease.
        let listing = self
            .listing_by_id
            .get(&listing_id)
            .expect("Listing Id for rent transfer does not exist!");
        let msg_rent_transfer_json = json!({
            "nft_contract_id":listing.nft_contract_id.clone(),
            "nft_token_id": listing.nft_token_id.clone(),
        })
        .to_string();

        // log rent transfer
        env::log_str(
            &json!({
                "type": "[INFO] NiFTyRent Marketplace: transfer rent",
                "params": {
                    "nft_contract_id": listing.nft_contract_id.clone(),
                    "nft_token_id": listing.nft_token_id.clone(),
                    "ft_contract": listing.ft_contract_id.clone(),
                    "price": listing.price.clone(),
                }
            })
            .to_string(),
        );

        ext_ft::ext(ft_contract_id.clone())
            .with_attached_deposit(1)
            .with_static_gas(Gas(3 * TGAS))
            .ft_transfer_call(
                self.rental_contract_id.clone(), // receiver_id
                amount,                          // amount
                memo,                            // memo
                msg_rent_transfer_json,
            );

        // remove the listing when both nft transfer and rent transfer succeeded
        self.internal_remove_listing(listing_id.clone());

        // refund set to 0
        let refund_ammount: U128 = U128::from(0);
        return refund_ammount;
    }

    #[private]
    pub fn create_listing_with_payout(
        &mut self,
        owner_id: AccountId,
        approval_id: u64,
        nft_contract_id: AccountId,
        nft_token_id: TokenId,
        ft_contract_id: AccountId,
        price: U128,
        lease_start_ts_nano: u64,
        lease_end_ts_nano: u64,
    ) {
        // log the request to create a listing
        env::log_str(
            &json!({
                "type": "[DEBUG] NiFTyRent Marketplace: Create a listing for the leasing NFT.",
                "params": {
                    "nft_contract_id": nft_contract_id.clone(),
                    "nft_token_id": nft_token_id.clone(),
                    "lender": owner_id.clone(),
                }
            })
            .to_string(),
        );

        let optional_payout;
        if is_promise_success() {
            // If NFT has implemented the `nft_payout` interface
            // then process the result and verify if sum of payout is close enough to the original price
            optional_payout = promise_result_as_success().map(|value| {
                let payout = serde_json::from_slice::<Payout>(&value).unwrap();
                let payout_diff: u128 = price
                    .0
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
                    "The difference between the listing price and the sum of payout is too large."
                );
                payout
            });
        } else {
            // If leased nft didn't provide payouts, we add a proxy payout record making original lender own all the rent.
            // This will make claiming back using LEASE NFT easier.
            optional_payout = Some(Payout {
                payout: HashMap::from([(owner_id.clone(), U128::from(price.clone()))]),
            });
        }

        // build the listing
        let new_listing: Listing = Listing {
            owner_id: owner_id,
            approval_id: approval_id,
            nft_contract_id: nft_contract_id.clone(),
            nft_token_id: nft_token_id.clone(),
            ft_contract_id: ft_contract_id,
            price: price,
            lease_start_ts_nano: lease_start_ts_nano,
            lease_end_ts_nano: lease_end_ts_nano,
            payout: optional_payout,
        };

        self.internal_insert_listing(&new_listing);
    }
    // ------------------ Internal Helpers -----------------

    fn internal_insert_listing(&mut self, listing_info: &Listing) {
        // create listing_id based on listing info
        let listing_id = (
            listing_info.nft_contract_id.clone(),
            listing_info.nft_token_id.clone(),
        );

        self.listing_by_id.insert(&listing_id, &listing_info);

        // Update the index: listing_ids_by_owner_id
        let mut listing_ids_set = self
            .listing_ids_by_owner_id
            .get(&listing_info.owner_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::ListingsByOwnerIdInner {
                    account_id_hash: hash_account_id(&listing_info.owner_id),
                })
            });
        listing_ids_set.insert(&listing_id);
        self.listing_ids_by_owner_id
            .insert(&listing_info.owner_id, &listing_ids_set);

        // Update the index: listing_ids_by_NFT_contract_id
        let mut listing_ids_set = self
            .listing_ids_by_nft_contract_id
            .get(&listing_info.nft_contract_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::ListingsByNftContractIdInner {
                    account_id_hash: hash_account_id(&listing_info.nft_contract_id),
                })
            });
        listing_ids_set.insert(&listing_id);
        self.listing_ids_by_nft_contract_id
            .insert(&listing_info.nft_contract_id, &listing_ids_set);

        // TODO(steven): remove this logging or find out why it breaks when running on testnet.
        // env::log_str(
        //     &json!({
        //         "type": "insert_listing",
        //         "params": {
        //             "owner_id": owner_id,
        //             "approval_id": approval_id,
        //             "nft_contract_id": nft_contract_id,
        //             "nft_token_id": nft_token_id,
        //             "ft_contract_id": ft_contract_id,
        //             "price": price,
        //             "lease_start_ts_nano": lease_start_ts_nano,
        //             "lease_end_ts_nano": lease_end_ts_nano,
        //         }
        //     })
        //     .to_string(),
        // );
    }

    fn internal_remove_listing(&mut self, listing_id: ListingId) {
        // check if the target listing exist
        let listing = self
            .listing_by_id
            .get(&listing_id)
            .expect("Input listing_id does not exist");

        // remove the record in listing_by_id index
        self.listing_by_id.remove(&listing_id);

        // remove from index: listing_ids_by_owner_id
        let mut listing_id_set = self.listing_ids_by_owner_id.get(&listing.owner_id).unwrap();
        listing_id_set.remove(&listing_id);

        if listing_id_set.is_empty() {
            self.listing_ids_by_owner_id.remove(&listing.owner_id);
        } else {
            self.listing_ids_by_owner_id
                .insert(&listing.owner_id, &listing_id_set);
        }

        // remove from index: listing_ids_by_NFT_contract_id
        let mut listing_id_set = self
            .listing_ids_by_nft_contract_id
            .get(&listing.nft_contract_id)
            .unwrap();
        listing_id_set.remove(&listing_id);

        if listing_id_set.is_empty() {
            self.listing_ids_by_nft_contract_id
                .remove(&listing.nft_contract_id);
        } else {
            self.listing_ids_by_nft_contract_id
                .insert(&listing.nft_contract_id, &listing_id_set);
        }

        // log the listing removal
        env::log_str(
            &json!({
                "type": "[INFO] NiFTyRent Marketplace: remove listing",
                "params": {
                    "listing_id": &listing_id,
                    "owner_id": listing.owner_id,
                    "nft_contract_id": listing.nft_contract_id,
                    "nft_token_id": listing.nft_token_id,
                }
            })
            .to_string(),
        );
    }

    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner_id,
            "This function can only be called by the owner!"
        )
    }
}

/// Helper function to add some account ids to a given set.
fn insert_accounts(accounts: Vec<AccountId>, set: &mut UnorderedSet<AccountId>) {
    accounts.iter().for_each(|id| {
        set.insert(id);
    });
}

/// Helper function to remove some account ids to a given set.
fn remove_accounts(accounts: Vec<AccountId>, set: &mut UnorderedSet<AccountId>) {
    accounts.iter().for_each(|id| {
        set.remove(id);
    });
}

fn hash_account_id(account_id: &AccountId) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(account_id.as_bytes()));
    hash
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
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, PromiseResult, RuntimeFeesConfig, VMConfig};

    #[test]
    fn test_new() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let contract = Contract::new(owner_id, treasury_id, rental_contract_id);
        assert_eq!(accounts(1), contract.owner_id);
        assert_eq!(accounts(2), contract.treasury_id);
        assert_eq!(accounts(3), contract.rental_contract_id);

        assert_eq!(0, contract.list_allowed_ft_contract_ids().len());
        assert_eq!(0, contract.list_allowed_nft_contract_ids().len());
    }

    #[test]
    fn test_list_allowed_ft_contract_ids_succeed() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id, treasury_id, rental_contract_id);
        let ft_contract_id: AccountId = accounts(4).into();
        contract
            .allowed_ft_contract_ids
            .insert(&ft_contract_id.clone());
        assert_eq!(ft_contract_id, contract.list_allowed_ft_contract_ids()[0]);
    }

    #[test]
    fn test_list_allowed_nft_contract_ids_succeed() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id, treasury_id, rental_contract_id);
        let nft_contract_id: AccountId = accounts(4).into();
        contract
            .allowed_nft_contract_ids
            .insert(&nft_contract_id.clone());
        assert_eq!(nft_contract_id, contract.list_allowed_nft_contract_ids()[0]);
    }

    #[test]
    fn test_list_listings_by_owner_id_succeed() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id.clone(), treasury_id, rental_contract_id);

        let listing_owner_id: AccountId = accounts(5).into();
        let approval_id: u64 = 1;
        let nft_contract_id: AccountId = accounts(2).into();
        let nft_token_id: TokenId = "test_token".to_string();
        let ft_contract_id: AccountId = accounts(3).into();
        let price: U128 = U128(100);
        // Monday, March 27, 2023 2:32:10 AM
        let lease_start_ts_nano: u64 = 1679884330000000000;
        // Tuesday, March 28, 2023 2:32:10 AM
        let lease_end_ts_nano: u64 = 1679970730000000000;

        // build the listing
        let new_listing: Listing = Listing {
            owner_id: listing_owner_id.clone(),
            approval_id: approval_id.clone(),
            nft_contract_id: nft_contract_id.clone(),
            nft_token_id: nft_token_id.clone(),
            ft_contract_id: ft_contract_id.clone(),
            price: price.clone(),
            lease_start_ts_nano: lease_start_ts_nano.clone(),
            lease_end_ts_nano: lease_end_ts_nano.clone(),
            payout: None,
        };

        contract.internal_insert_listing(&new_listing);

        let res = contract.list_listings_by_owner_id(listing_owner_id.clone());
        assert_eq!(1, res.len());
        assert_eq!(listing_owner_id, res[0].owner_id);
    }

    #[test]
    fn test_list_listings_by_owner_id_id_not_found() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id.clone(), treasury_id, rental_contract_id);

        let listing_owner_id: AccountId = accounts(5).into();
        let approval_id: u64 = 1;
        let nft_contract_id: AccountId = accounts(2).into();
        let nft_token_id: TokenId = "test_token".to_string();
        let ft_contract_id: AccountId = accounts(3).into();
        let price: U128 = U128(100);
        // Monday, March 27, 2023 2:32:10 AM
        let lease_start_ts_nano: u64 = 1679884330000000000;
        // Tuesday, March 28, 2023 2:32:10 AM
        let lease_end_ts_nano: u64 = 1679970730000000000;

        // build the listing
        let new_listing: Listing = Listing {
            owner_id: listing_owner_id.clone(),
            approval_id: approval_id.clone(),
            nft_contract_id: nft_contract_id.clone(),
            nft_token_id: nft_token_id.clone(),
            ft_contract_id: ft_contract_id.clone(),
            price: price.clone(),
            lease_start_ts_nano: lease_start_ts_nano.clone(),
            lease_end_ts_nano: lease_end_ts_nano.clone(),
            payout: None,
        };

        contract.internal_insert_listing(&new_listing);

        let res = contract.list_listings_by_owner_id(accounts(1).into());
        assert_eq!(0, res.len());
    }

    #[test]
    fn test_list_listings_by_nft_contract_id_succeed() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id.clone(), treasury_id, rental_contract_id);

        let listing_owner_id: AccountId = accounts(5).into();
        let approval_id: u64 = 1;
        let nft_contract_id: AccountId = accounts(2).into();
        let nft_token_id: TokenId = "test_token".to_string();
        let ft_contract_id: AccountId = accounts(3).into();
        let price: U128 = U128(100);
        // Monday, March 27, 2023 2:32:10 AM
        let lease_start_ts_nano: u64 = 1679884330000000000;
        // Tuesday, March 28, 2023 2:32:10 AM
        let lease_end_ts_nano: u64 = 1679970730000000000;

        // build the listing
        let new_listing: Listing = Listing {
            owner_id: listing_owner_id.clone(),
            approval_id: approval_id.clone(),
            nft_contract_id: nft_contract_id.clone(),
            nft_token_id: nft_token_id.clone(),
            ft_contract_id: ft_contract_id.clone(),
            price: price.clone(),
            lease_start_ts_nano: lease_start_ts_nano.clone(),
            lease_end_ts_nano: lease_end_ts_nano.clone(),
            payout: None,
        };

        contract.internal_insert_listing(&new_listing);

        let res = contract.list_listings_by_nft_contract_id(nft_contract_id.clone());
        assert_eq!(1, res.len());
        assert_eq!(nft_contract_id, res[0].nft_contract_id);
    }

    #[test]
    fn test_list_listings_by_nft_contract_id_id_not_found() {
        let owner_id: AccountId = accounts(1).into();
        let treasury_id: AccountId = accounts(2).into();
        let rental_contract_id: AccountId = accounts(3).into();

        let mut contract = Contract::new(owner_id.clone(), treasury_id, rental_contract_id);

        let listing_owner_id: AccountId = accounts(5).into();
        let approval_id: u64 = 1;
        let nft_contract_id: AccountId = accounts(2).into();
        let nft_token_id: TokenId = "test_token".to_string();
        let ft_contract_id: AccountId = accounts(3).into();
        let price: U128 = U128(100);
        // Monday, March 27, 2023 2:32:10 AM
        let lease_start_ts_nano: u64 = 1679884330000000000;
        // Tuesday, March 28, 2023 2:32:10 AM
        let lease_end_ts_nano: u64 = 1679970730000000000;

        // build the listing
        let new_listing: Listing = Listing {
            owner_id: listing_owner_id.clone(),
            approval_id: approval_id.clone(),
            nft_contract_id: nft_contract_id.clone(),
            nft_token_id: nft_token_id.clone(),
            ft_contract_id: ft_contract_id.clone(),
            price: price.clone(),
            lease_start_ts_nano: lease_start_ts_nano.clone(),
            lease_end_ts_nano: lease_end_ts_nano.clone(),
            payout: None,
        };

        contract.internal_insert_listing(&new_listing);

        let res = contract.list_listings_by_nft_contract_id(accounts(3).into());
        assert_eq!(0, res.len());
    }

    #[test]
    fn test_create_listing_with_payout_succeeds_when_nft_payout_xcc_succeeded() {
        let marketplace_owner_id: AccountId = create_a_dummy_account_id("marketplace_owner");
        let treasury_id: AccountId = create_a_dummy_account_id("treasury_owner");
        let rental_contract_id: AccountId = create_a_dummy_account_id("rental_contract_owner");

        let mut contract = Contract::new(
            marketplace_owner_id.clone(),
            treasury_id,
            rental_contract_id,
        );

        let nft_contract_id: AccountId = create_a_dummy_account_id("nft_contract");
        let nft_token_id: TokenId = "test_token".to_string();
        let nft_token_owner_id: AccountId = create_a_dummy_account_id("nft_token_owner");
        let ft_contract_id: AccountId = create_a_dummy_account_id("ft_contract_id");
        let price: U128 = U128::from(5);

        let payout_expected = Payout {
            payout: HashMap::from([
                (accounts(2).into(), U128::from(1)),
                (accounts(3).into(), U128::from(4)),
            ]),
        };

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(nft_token_owner_id.clone())
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(
                serde_json::to_vec(&payout_expected).unwrap()
            )],
        );

        contract.create_listing_with_payout(
            nft_token_owner_id.clone(),
            1, // dummy approval id
            nft_contract_id.clone(),
            nft_token_id.clone(),
            ft_contract_id.clone(),
            price,
            0,
            1000,
        );

        assert!(!contract.listing_by_id.is_empty());
        assert!(!contract
            .list_listings_by_nft_contract_id(nft_contract_id.clone())
            .is_empty());

        let listing_info = &contract.list_listings_by_owner_id(nft_token_owner_id.clone())[0];
        assert_eq!(nft_contract_id, listing_info.nft_contract_id);
        assert_eq!(nft_token_id, listing_info.nft_token_id);
        assert_eq!(nft_token_owner_id, listing_info.owner_id);
        assert_eq!(Some(payout_expected), listing_info.payout);
        assert_eq!(5, listing_info.price.0);
        assert_eq!(1000, listing_info.lease_end_ts_nano);
    }

    #[test]
    fn test_create_listing_with_payout_succeeds_when_nft_payout_xcc_failed() {
        // When nft_payout xcc failed, we should still produce a internal payout record,
        // allocating to the original lender the whole price.

        let marketplace_owner_id: AccountId = create_a_dummy_account_id("marketplace_owner");
        let treasury_id: AccountId = create_a_dummy_account_id("treasury_owner");
        let rental_contract_id: AccountId = create_a_dummy_account_id("rental_contract_owner");

        let mut contract = Contract::new(
            marketplace_owner_id.clone(),
            treasury_id,
            rental_contract_id,
        );

        let nft_contract_id: AccountId = create_a_dummy_account_id("nft_contract");
        let nft_token_id: TokenId = "test_token".to_string();
        let nft_token_owner_id: AccountId = create_a_dummy_account_id("nft_token_owner");
        let ft_contract_id: AccountId = create_a_dummy_account_id("ft_contract_id");
        let price: U128 = U128::from(5);

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(nft_token_owner_id.clone())
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Failed],
        );

        contract.create_listing_with_payout(
            nft_token_owner_id.clone(),
            1, // dummy approval id
            nft_contract_id.clone(),
            nft_token_id.clone(),
            ft_contract_id.clone(),
            price,
            0,
            1000,
        );

        assert!(!contract.listing_by_id.is_empty());
        assert!(!contract
            .list_listings_by_nft_contract_id(nft_contract_id.clone())
            .is_empty());

        let payout_expected = Payout {
            payout: HashMap::from([(nft_token_owner_id.clone().into(), U128::from(price.clone()))]),
        };
        let listing_info = &contract.list_listings_by_owner_id(nft_token_owner_id.clone())[0];

        assert!(listing_info.payout.is_some());
        assert_eq!(1, listing_info.payout.as_ref().unwrap().payout.keys().len());
        assert_eq!(Some(payout_expected), listing_info.payout);
    }

    #[test]
    #[should_panic(
        expected = "The difference between the listing price and the sum of payout is too large."
    )]
    fn test_create_listing_with_payout_failed_invalid_payout() {
        let marketplace_owner_id: AccountId = create_a_dummy_account_id("marketplace_owner");
        let treasury_id: AccountId = create_a_dummy_account_id("treasury_owner");
        let rental_contract_id: AccountId = create_a_dummy_account_id("rental_contract_owner");

        let mut contract = Contract::new(
            marketplace_owner_id.clone(),
            treasury_id,
            rental_contract_id,
        );

        let nft_contract_id: AccountId = create_a_dummy_account_id("nft_contract");
        let nft_token_id: TokenId = "test_token".to_string();
        let nft_token_owner_id: AccountId = create_a_dummy_account_id("nft_token_owner");
        let ft_contract_id: AccountId = create_a_dummy_account_id("ft_contract_id");
        let price: U128 = U128::from(5);

        // payout is hard coded and its add up differs from asking price
        let payout_returned = Payout {
            payout: HashMap::from([
                (nft_token_owner_id.clone(), U128::from(1)),
                (accounts(3).into(), U128::from(2)),
            ]),
        };

        testing_env!(
            VMContextBuilder::new()
                .current_account_id(accounts(0))
                .predecessor_account_id(nft_token_owner_id.clone())
                .build(),
            VMConfig::test(),
            RuntimeFeesConfig::test(),
            HashMap::default(),
            vec![PromiseResult::Successful(
                serde_json::to_vec(&payout_returned).unwrap()
            )],
        );

        contract.create_listing_with_payout(
            nft_token_owner_id.clone(),
            1, // dummy approval id
            nft_contract_id.clone(),
            nft_token_id.clone(),
            ft_contract_id.clone(),
            price,
            0,
            1000,
        );
    }

    // Helper function to generate a dummy AccountId using input name
    pub(crate) fn create_a_dummy_account_id(account_name: &str) -> AccountId {
        AccountId::new_unchecked(account_name.to_string())
    }

    // ===== Unit Test =====
    // TODO: test_add_allowed_ft_contract_ids_succeeds
    // TODO: test_add_allowed_nft_contract_ids_succeeds
    // TODO: test_add_allowed_ft_contract_ids_fails_wrong_caller
    // TODO: test_add_allowed_nft_contract_ids_fails_wrong_caller
    // TODO: test_remove_allowed_ft_contract_ids
    // TODO: test_remove_allowed_nft_contract_ids
}
