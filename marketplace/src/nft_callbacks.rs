use crate::*;
/// approval callbacks from NFT Contracts
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ListingJson {
    pub price: U128,
    pub ft_contract_id: AccountId,
    pub lease_start_time: U64,
    pub lease_end_time: U64,
}

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
        // enforce cross contract call and owner_id is signer

        let nft_contract_id = env::predecessor_account_id();
        let signer_id = env::signer_account_id();
        assert_ne!(
            env::current_account_id(),
            nft_contract_id,
            "nft_on_approve should only be called via XCC"
        );
        assert_eq!(owner_id, signer_id, "owner_id should be signer_id");

        assert!(
            self.allowed_nft_contract_ids.contains(&nft_contract_id),
            "nft_contract_id is not approved"
        );

        let ListingJson {
            price,
            ft_contract_id,
            lease_start_time,
            lease_end_time,
        } = near_sdk::serde_json::from_str(&msg).expect("Invalid ListingJson");

        self.internal_delete_market_data(&nft_contract_id, &token_id);

        if self.allowed_ft_token_ids.contains(&ft_contract_id) != true {
            env::panic_str(&"ft_contract_id not allowed");
        }

        self.internal_insert_listing(
            owner_id,
            approval_id,
            nft_contract_id,
            token_id,
            ft_contract_id,
            price,
            lease_start_time,
            lease_end_time,
        );
    }
}
