use crate::*;

/// This file includes NFT related features but not required in the Nomicon Standards

// #[near_bindgen]
 impl Contract {
    /// returns the total number of active leases
    /// useful for nft_total_supply() in IOU nft
    pub(crate) fn total_active_leases(&mut self) -> u128{
        todo!()
    }

    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        memo: Option<String>
    ) -> Token {
        // 1. update IOU token owner to new owner
        // 2. update lease condition to reflect the lender change
        todo!()
    }

    /// Mint a new IOU token. It will be called once lease become active to mint a new IOU token.
    /// This function is visible only within the current contract,
    /// No other accounts can mint the IOU token
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