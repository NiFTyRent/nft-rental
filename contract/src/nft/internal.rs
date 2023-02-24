use crate::*;
use near_contract_standards::non_fungible_token::events::{NftMint, NftTransfer};
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
        let lease_id = self.lease_token_id_to_lease_id(token_id);
        let lease_condition = self
            .lease_map
            .get(&lease_id)
            .expect("No matching lease for the given LEASE token id!");
        let owner_id = lease_condition.lender_id.clone();
        assert_eq!(&owner_id, sender_id, "Only current lender can transfer!");
        assert_ne!(
            &owner_id, receiver_id,
            "Current lender can not be the receiver!"
        );

        // Transfer lease from sender to receiver
        self.internal_update_active_lease_lender(sender_id, receiver_id, &lease_id);

        // Log transfer event as per the Events standard
        NftTransfer {
            old_owner_id: sender_id,
            new_owner_id: receiver_id,
            token_ids: &[token_id],
            authorized_id: None, // approval is not supported at the moment
            memo: memo.as_deref(),
        }
        .emit();

        // Return the new token info, when internal transfer succeeded
        Token {
            token_id: token_id.clone(),
            owner_id: receiver_id.clone(),
            metadata: None,
            approved_account_ids: None,
        }
    }

    /// Update NFT related fields. It will be called once lease become active.
    /// In essence, this function updates indices that tracks active lease.
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

        // Log mint event as per the Events standard
        let token_id = self.lease_id_to_lease_token_id(&lease_id);
        NftMint {
            owner_id: &receiver_id,
            token_ids: &[&token_id],
            memo: None,
        }
        .emit();
    }

    pub(crate) fn lease_token_id_to_lease_id(&self, token_id: &TokenId) -> LeaseId {
        let splits: Vec<&str> = token_id.split("_lender").collect();
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

    use crate::tests::*;
    use crate::{Contract, LeaseId, LeaseState};

    use near_contract_standards::non_fungible_token::TokenId;
    use near_sdk::test_utils::{self, accounts};

    #[test]
    fn test_lease_id_to_lease_token_id_succeeds() {
        let lease_id: LeaseId = "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK".to_string();
        let lease_token_id_expected: TokenId =
            "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK_lender".to_string();

        let contract = Contract::new(accounts(1).into());
        let lease_token_id_real: TokenId = contract.lease_id_to_lease_token_id(&lease_id);

        assert_eq!(lease_token_id_expected, lease_token_id_real);
    }

    #[test]
    fn test_lease_token_id_to_lease_id_succeeds() {
        let lease_token_id: TokenId =
            "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK_lender".to_string();
        let lease_id_expected: LeaseId = "8Vin66zVuhiB6tb9Zn9P6vRJpjQMEUMum1EkKESxJnK".to_string();

        let contract = Contract::new(accounts(1).into());
        let lease_id_real: LeaseId = contract.lease_token_id_to_lease_id(&lease_token_id);

        assert_eq!(lease_id_expected, lease_id_real);
    }

    /// check the indices got updated correctly
    /// - active_lease_ids got updated
    /// - active_lease_ids_by_lender has new record
    #[test]
    fn test_nft_mint_succeeds() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);
        lease_condition.state = LeaseState::Active;

        // Before calling nft mint, no records for the active lease
        assert!(!contract.active_lease_ids.contains(&lease_key));
        assert!(!contract
            .active_lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));

        contract.nft_mint(lease_key.clone(), lease_condition.lender_id.clone());

        // After calling nft_mint(), active lease records should be updated
        assert!(contract.active_lease_ids.contains(&lease_key));
        assert!(contract
            .active_lease_ids_by_lender
            .contains_key(&lease_condition.lender_id));
        assert_eq!(
            contract.lease_map.get(&lease_key).unwrap().lender_id,
            lease_condition.lender_id
        );
        assert!(contract
            .active_lease_ids_by_lender
            .get(&lease_condition.lender_id)
            .unwrap()
            .contains(&lease_key));
    }

    #[test]
    fn test_event_mint_log_succeeds() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.lender_id = get_dummy_account_id("alice");

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);
        lease_condition.state = LeaseState::Active;

        contract.nft_mint(lease_key.clone(), lease_condition.lender_id.clone());

        // Check logs output correctly
        let mint_log = &test_utils::get_logs()[0];
        let mint_log_expected = r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"alice","token_ids":["test_key_lender"]}]}"#;
        assert_eq!(mint_log, mint_log_expected);
    }

    #[test]
    fn test_event_transfer_log_for_nft_transfer_succeeds() {
        let mut contract = Contract::new(accounts(0).into());
        let mut lease_condition = create_lease_condition_default();
        lease_condition.lender_id = get_dummy_account_id("alice");

        let lease_key = "test_key".to_string();
        contract.internal_insert_lease(&lease_key, &lease_condition);
        lease_condition.state = LeaseState::Active;

        let token_id = contract.lease_id_to_lease_token_id(&lease_key);
        contract.nft_mint(lease_key.clone(), lease_condition.lender_id.clone());
        contract.internal_transfer(
            &lease_condition.lender_id,
            &get_dummy_account_id("bob"),
            &token_id,
            None,
        );
        
        // Check logs emit correctly
        let transfer_log = &test_utils::get_logs()[1];
        let transfer_log_expected = r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_transfer","data":[{"old_owner_id":"alice","new_owner_id":"bob","token_ids":["test_key_lender"]}]}"#;
        assert_eq!(transfer_log, transfer_log_expected);
    }
}
