use crate::*;

/// This file includes NFT related features but not required in the Nomicon Standards

// #[near_bindgen]
impl Contract {
    /// returns the total number of active leases
    /// useful for nft_total_supply() in IOU nft
    pub(crate) fn total_active_leases(&mut self) -> u128 {
        todo!()
    }

    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        memo: Option<String>,
    ) -> Token {
        // 1. update IOU token owner to new owner
        // 2. update lease condition to reflect the lender change
        todo!()
    }

    /// Mint a new IOU token. It will be called once lease become active to mint a new IOU token.
    /// This function is visible only within the current contract,
    /// No other accounts can mint the IOU token
    pub(crate) fn nft_mint(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
        receiver_id: AccountId,
    ) {
        // update the record for token_ids_per_owner
        let mut token_ids_set = self
            .token_ids_per_owner
            .get(&receiver_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(
                    StorageKey::TokenIdsPerOwnerInner {
                        // get a new unique prefix for the collection by hashing owner
                        account_id_hash: hash_account_id(&receiver_id),
                    }
                    .try_to_vec()
                    .unwrap(),
                )
            });

        // insert the token id and metadata
        self.token_metadata_by_id.insert(&token_id, &metadata);

        token_ids_set.insert(&token_id);
        self.token_ids_per_owner
            .insert(&receiver_id, &token_ids_set);

        //add token_owner and token_id can be inferred by lease_ids_by_lender.
    }
}