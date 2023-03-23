use crate::*;
/// approval callbacks from NFT Contracts

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ListingJson {
    ft_contract_id: AccountId,
    price: U128,
    lease_start_ts_nano: U64,
    lease_end_ts_nano: U64,
}

/**
 * Trait to be used as the call back from NFT contract for listing creation.
 * When a lender trys to create a listing, she calls nft_approve attaching a msg of required info.
 * NFT contract will fire a XCC to this marketplace to invoke this function.
 * This will triger creating a listing.
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
    /// Function to initiate new listing creation.
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: u64,
        msg: String,
    ) {
        // enforce cross contract call
        let nft_contract_id = env::predecessor_account_id();
        assert_ne!(
            env::current_account_id(),
            nft_contract_id,
            "nft_on_approve should only be called via XCC"
        );

        // enforce owner_id is signer
        let signer_id = env::signer_account_id();
        assert_eq!(owner_id, signer_id, "owner_id should be signer_id");

        // enforce nft contract is allowed
        assert!(
            self.allowed_nft_contract_ids.contains(&nft_contract_id),
            "nft_contract_id is not allowed!"
        );

        let listing_json: ListingJson =
            near_sdk::serde_json::from_str(&msg).expect("Invalid Listing Json!");

        // enforce ft contract is allowed
        assert!(
            self.allowed_ft_contract_ids
                .contains(&listing_json.ft_contract_id),
            "ft_contract_id is not allowed!"
        );

        // TODO(syu): Do we need this step?
        self.internal_delete_market_data(&nft_contract_id, &token_id);

        // record a listing
        self.internal_insert_listing(
            owner_id,
            approval_id,
            nft_contract_id,
            token_id,
            listing_json.ft_contract_id,
            listing_json.price,
            listing_json.lease_start_ts_nano.0,
            listing_json.lease_end_ts_nano.0,
        );
    }
}
