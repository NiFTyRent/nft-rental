use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshDeserialize, BorshSerialize},
    bs58,
    collections::{LookupMap, UnorderedMap, UnorderedSet},
    env, ext_contract, is_promise_success,
    json_types::{U128, U64},
    near_bindgen, require,
    serde::{Deserialize, Serialize},
    serde_json::json,
    AccountId, BorshStorageKey, CryptoHash, Gas, PanicOnDefault,
};

mod externals;
mod ft_callbacks;
mod nft_callbacks;
use crate::externals::*;

pub const TGAS: u64 = 1_000_000_000_000;

type ListingId = String;

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
    pub listings: UnorderedMap<ListingId, Listing>,
    /// Whitelist of FT contracts for rent payment.
    pub allowed_ft_contract_ids: UnorderedSet<AccountId>,
    // TODO(libo): Shops?
    pub allowed_nft_contract_ids: UnorderedSet<AccountId>,

    // TODO: do we need it?
    /// Indices of listing for quick lookup.
    pub listings_by_owner_id: LookupMap<AccountId, UnorderedSet<ListingId>>,
    pub listings_by_nft_contract_id: LookupMap<AccountId, UnorderedSet<ListingId>>,
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
            listings: UnorderedMap::new(StorageKey::Listings),
            allowed_ft_contract_ids: UnorderedSet::new(StorageKey::FTTokenIds),
            allowed_nft_contract_ids: UnorderedSet::new(StorageKey::NFTContractIds),
            listings_by_owner_id: LookupMap::new(StorageKey::ListingsByOwnerId),
            listings_by_nft_contract_id: LookupMap::new(StorageKey::ListingsByNftContractId),
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
        todo!();
    }
    /// List all FT contract that are allowed to be used for payment.
    pub fn list_allowed_ft_contract_ids(&self) -> Vec<AccountId> {
        todo!();
    }

    pub fn list_listings_by_owner_id(&self, owner_id: AccountId) -> Vec<Listing> {
        todo!();
    }

    pub fn list_listings_by_nft_contract_id(&self, nft_contract_id: AccountId) -> Vec<Listing> {
        todo!();
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
    ) -> U128 {
        require!(
            is_promise_success(),
            "NFT transfer failed. Abort rent transfer!"
        );

        // Trasnfer the rent to Core contract.
        // TODO(syu): do we need to check the target lease got created successfully? This will need to call ft_on_transfer(). Also a map between listing_id and lease_id
        ext_ft::ext(ft_contract_id.clone())
            .with_attached_deposit(1)
            .with_static_gas(Gas(10 * TGAS))
            .ft_transfer(
                self.rental_contract_id.clone(), // receiver_id
                amount,                          // amount
                memo,                            // memo
            );

        // refund set to 0
        let refund_ammount: U128 = U128::from(0);
        return refund_ammount;
    }

    // ------------------ Internal Helpers -----------------

    fn internal_insert_listing(
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
        // create listing_id
        let seed = near_sdk::env::random_seed();
        let listing_id = bs58::encode(seed)
            .with_alphabet(bs58::Alphabet::BITCOIN)
            .into_string();

        self.listings.insert(
            &listing_id,
            &Listing {
                owner_id: owner_id.clone(),
                approval_id,
                nft_contract_id: nft_contract_id.clone(),
                nft_token_id: nft_token_id.clone(),
                ft_contract_id: ft_contract_id.clone(),
                price: price.into(),
                lease_start_ts_nano,
                lease_end_ts_nano,
            },
        );

        // Update the listings by owner id index
        let mut listing_ids_set = self.listings_by_owner_id.get(&owner_id).unwrap_or_else(|| {
            UnorderedSet::new(StorageKey::ListingsByOwnerIdInner {
                account_id_hash: hash_account_id(&owner_id),
            })
        });
        listing_ids_set.insert(&listing_id);
        self.listings_by_owner_id
            .insert(&owner_id, &listing_ids_set);

        // Update the listings by NFT contract id index
        let mut listing_ids_set = self
            .listings_by_nft_contract_id
            .get(&nft_contract_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(StorageKey::ListingsByNftContractIdInner {
                    account_id_hash: hash_account_id(&nft_contract_id),
                })
            });
        listing_ids_set.insert(&listing_id);
        self.listings_by_nft_contract_id
            .insert(&nft_contract_id, &listing_ids_set);

        env::log_str(
            &json!({
                "type": "insert_listing",
                "params": {
                    "owner_id": owner_id,
                    "approval_id": approval_id,
                    "nft_contract_id": nft_contract_id,
                    "nft_token_id": nft_token_id,
                    "ft_contract_id": ft_contract_id,
                    "price": price,
                    "lease_start_ts_nano": lease_start_ts_nano,
                    "lease_end_ts_nano": lease_end_ts_nano,
                }
            })
            .to_string(),
        );
    }

    fn internal_remove_listing(&mut self, listing_id: ListingId) {
        todo!()
    }

    fn internal_delete_market_data(&mut self, nft_contract_id: &AccountId, token_id: &TokenId) {
        todo!()
    }

    fn assert_owner(&self) {
        todo!()
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
