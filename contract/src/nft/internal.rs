use crate::*;
use near_contract_standards::non_fungible_token::Token;

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

        // Transfer lease from sender to receiver
        self.internal_update_active_lease_lender(sender_id, receiver_id, token_id);

        // If there was memo, log it
        if let Some(memo) = memo {
            env::log_str(&format!("Memo: {}", memo).to_string());
        }

        // Return the new token info, when internal transfer succeeded
        Token {
            token_id: token_id.clone(),
            owner_id: receiver_id.clone(),
            metadata: None,
            approved_account_ids: None,
        }
    }

    /// Update NFT related fields. It will be called once lease become active.
    /// This function is visible only within the current contract
    pub(crate) fn nft_mint(&mut self, lease_id: LeaseId, receiver_id: AccountId) {
        // Update the record for active_leases
        let mut active_lease_ids_set = self
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

        active_lease_ids_set.insert(&lease_id);
        self.active_lease_ids_by_lender
            .insert(&receiver_id, &active_lease_ids_set);

        // Record active leases/Lease Tokens
        self.active_lease_ids.insert(&lease_id);
    }

    pub(crate) fn lease_token_id_to_lease_id(&self, token_id: &TokenId) -> LeaseId {
        let splits: Vec<&str> = token_id.split('_').collect();
        splits[0].to_string()
    }

    pub(crate) fn lease_id_to_lease_token_id(&self, lease_id: &LeaseId) -> TokenId {
        let suffix: &str = "_lender";
        format!("{}{}", lease_id, suffix)
    }
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

    use near_contract_standards::non_fungible_token::TokenId;
    use near_sdk::test_utils::accounts;

    use crate::Contract;
    use crate::LeaseId;

    #[test]
    fn test_lease_id_to_lease_token_id_success() {
        let lease_id: LeaseId = "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK".to_string();
        let lease_token_id_expected: TokenId =
            "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK_lender".to_string();

        let contract = Contract::new(accounts(1).into());
        let lease_token_id_real: TokenId = contract.lease_id_to_lease_token_id(&lease_id);

        assert_eq!(lease_token_id_expected, lease_token_id_real);
    }

    #[test]
    fn test_lease_token_id_to_lease_id_success() {
        let lease_token_id: TokenId =
            "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK_lender".to_string();
        let lease_id_expected: LeaseId = "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK".to_string();

        let contract = Contract::new(accounts(1).into());
        let lease_id_real: LeaseId = contract.lease_token_id_to_lease_id(&lease_token_id);

        assert_eq!(lease_id_expected, lease_id_real);
    }
}
