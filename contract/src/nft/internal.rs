use crate::*;

/// This file includes NFT related features but not required in the Nomicon Standards

// #[near_bindgen]
impl Contract {
    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        memo: Option<String>,
    ) -> Token {
        // Check if the lease exist
        let lease_condition = self
            .lease_map
            .get(&token_id)
            .expect("No matching lease for the given LEASE token id!");
        let owner_id = lease_condition.lender_id.clone();
        assert_eq!(&owner_id, sender_id, "Only current lender can transfer!");
        assert_ne!(
            &owner_id, receiver_id,
            "Current lender can not be the receiver!"
        );

        // transfer lease from sender to receiver
        self.internal_update_active_lease_lender(sender_id, receiver_id, token_id);

        // if there was memo, log it
        if let Some(memo) = memo {
            env::log_str(&format!("Memo: {}", memo).to_string());
        }

        // return the new token info, when internal transfer succeeded
        Token {
            token_id: token_id.clone(),
            owner_id: receiver_id.clone(),
            metadata: None,
        }
    }

    /// This function updates only the lender info in an active lease
    /// All affecting indices will be updated
    pub(crate) fn internal_update_active_lease_lender(
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

        // 3. remove the active lease from the old lender
        // update index for active lease ids
        active_lease_ids_set.remove(lease_id);
        if active_lease_ids_set.is_empty() {
            self.active_lease_ids_by_lender.remove(old_lender);
        } else {
            self.active_lease_ids_by_lender
                .insert(old_lender, &active_lease_ids_set);
        }
        // update index for lease ids
        let mut lease_ids_set = self.lease_ids_by_lender.get(old_lender).unwrap();
        lease_ids_set.remove(lease_id);
        if lease_ids_set.is_empty() {
            self.lease_ids_by_lender.remove(old_lender);
        } else {
            self.lease_ids_by_lender.insert(old_lender, &lease_ids_set);
        }

        // 4. add the active lease to the new lender
        // update the index for active lease ids
        let mut active_lease_ids_set = self
            .active_lease_ids_by_lender
            .get(new_lender)
            .unwrap_or_else(|| {
                // if the receiver doesn't have any active lease, create a new record
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
        // udpate the index for lease ids
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

        // 5. update lease map index
        let mut lease_condition = self.lease_map.get(lease_id).unwrap();
        lease_condition.lender_id = new_lender.clone();
    }

    /// Update NFT related fields. It will be called once lease become active.
    /// This function is visible only within the current contract
    pub(crate) fn nft_mint(&mut self, token_id: TokenId, receiver_id: AccountId) {
        // update the record for active_leases
        let mut token_ids_set = self
            .active_lease_ids_by_lender
            .get(&receiver_id)
            .unwrap_or_else(|| {
                UnorderedSet::new(
                    StorageKey::ActiveLeaseIdsByOwnerInner {
                        // get a new unique prefix for the collection by hashing owner
                        account_id_hash: utils::hash_account_id(&receiver_id),
                    }
                    .try_to_vec()
                    .unwrap(),
                )
            });

        token_ids_set.insert(&token_id);
        self.active_lease_ids_by_lender
            .insert(&receiver_id, &token_ids_set);

        // Record active leases/Lease Tokens
        self.active_lease_ids.insert(&token_id);
    }
}
