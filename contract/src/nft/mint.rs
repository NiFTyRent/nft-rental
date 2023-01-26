use crate::*;
use near_contract_standards::non_fungible_token::refund_deposit;

#[near_bindgen]
impl Contract {
    #[payable]
    pub(crate) fn nft_mint(&mut self, token_id: TokenId, metadata: TokenMetadata, receiver_id: AccountId) {
        //measure the initial storage being used on the contract
        let initial_storage_usage = env::storage_usage();

        let token = Token {
            token_id: token_id,
            owner_id: receiver_id,
            metadata: None,
        };

        //insert the token ID and token struct, when the token doesn't exist
        assert!(
            self.tokens_by_id.insert(&token_id, &token).is_none(),
            "Token already exists"
        );

        //insert the token ID and metadata
        self.token_metadata_by_id.insert(&token_id, &metadata);

        //add token_id to its owner - This can be inferred by lease_ids_by_lender. No action here.

        //calculate the required storage
        let required_storage_in_bytes = env::storage_usage() - initial_storage_usage;

        //refund any excess storage if the user attached too much. Panic if they didn't attach enough to cover the required.
        refund_deposit(required_storage_in_bytes);
    }
}
