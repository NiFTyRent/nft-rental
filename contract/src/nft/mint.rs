use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub(crate) fn nft_mint(&mut self, token_id: TokenId, metadata: TokenMetadata, receiver_id: AccountId) {
        let token = Token {
            token_id: token_id.clone(),
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

    }
}
