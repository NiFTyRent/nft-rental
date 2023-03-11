use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{
    assert_one_yocto,
    borsh::{BorshDeserialize, BorshSerialize},
    collections::{LookupMap, UnorderedMap, UnorderedSet},
    env,
    json_types::{U128, U64},
    near_bindgen,
    serde::{Deserialize, Serialize},
    serde_json::json,
    AccountId, CryptoHash,
};

mod ft_callbacks;
mod nft_callbacks;

// TODO(libo): use a differnt id type, since same NFT could be listed for leased for different time period.
type ListingId = (AccountId, TokenId);

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
    pub lease_start_time: u64,
    pub lease_end_time: u64,
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

    pub fn set_treasury(&mut self, treasury_id: AccountId) {
        assert_one_yocto();
        self.assert_owner();
        self.treasury_id = treasury_id;
    }

    #[payable]
    pub fn add_approved_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        insert_accounts(nft_contract_ids, &mut self.approved_nft_contract_ids);
    }

    #[payable]
    pub fn remove_approved_nft_contract_ids(&mut self, nft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        remove_accounts(nft_contract_ids, &mut self.approved_nft_contract_ids);
    }

    #[payable]
    pub fn add_approved_ft_contract_ids(&mut self, ft_contract_ids: Vec<AccountId>) {
        self.assert_owner();
        insert_accounts(ft_contract_ids, &mut self.approved_ft_contract_ids);
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

    // ------------------ Internal Helpers -----------------

    fn internal_insert_listing(
        &mut self,
        owner_id: AccountId,
        approval_id: u64,
        nft_contract_id: AccountId,
        nft_token_id: TokenId,
        ft_contract_id: AccountId,
        price: U128,
        lease_start_time: u64,
        lease_end_time: u64,
    ) {
        let listing_id: ListingId = (nft_contract_id, nft_token_id);

        self.listings.insert(
            &listing_id
                & Listing {
                    owner_id: owner_id.clone(),
                    approval_id,
                    nft_contract_id: nft_contract_id.clone(),
                    nft_token_id: nft_token_id.clone(),
                    ft_contract_id: ft_contract_id.clone(),
                    price: price.into(),
                    lease_start_time,
                    lease_end_time,
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
                    "lease_start_time": lease_start_time,
                    "lease_end_time": lease_end_time,
                }
            })
            .to_string(),
        );
    }

    fn internal_remove_listing(&mut self, listing_id: ListingId) {
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
