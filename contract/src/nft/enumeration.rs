use crate::{nft::core::NonFungibleTokenCore, *};
use near_contract_standards::non_fungible_token::{
    enumeration::NonFungibleTokenEnumeration, Token,
};
use near_sdk::json_types::U128;

#[near_bindgen]
impl NonFungibleTokenEnumeration for Contract {
    /// Returns the total supply of non-fungible tokens as a string representing an
    /// Unsigned 128-bit integer to avoid JSON number limit of 2^53.
    fn nft_total_supply(&self) -> U128 {
        U128(self.active_lease_ids.len() as u128)
    }

    /// Get a list of all tokens.
    /// Returns an array of Token objects, for Pagination.
    fn nft_tokens(
        &self,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // default: unlimited (could fail due to gas limit)
    ) -> Vec<Token> {
        // Get starting index, default to 0
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            (self.active_lease_ids.len() as u128) >= start_index,
            "Out of bounds, please use a smaller from_index."
        );

        // Sainity check on limit
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);

        self.active_lease_ids
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|active_lease_id| {
                self.nft_token(self.lease_id_to_lease_token_id(&active_lease_id))
                    .unwrap()
            })
            .collect()
    }

    /// Get total NFT supply for a given account
    fn nft_supply_for_owner(&self, account_id: AccountId) -> U128 {
        let active_lease_ids_set = self.active_lease_ids_by_lender.get(&account_id);

        if let Some(active_lease_ids) = active_lease_ids_set {
            U128(active_lease_ids.len() as u128)
        } else {
            U128(0)
        }
    }

    /// Get list of all tokens owned by a given account
    fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // 10
    ) -> Vec<Token> {
        let active_lease_ids_per_owner_set = self.active_lease_ids_by_lender.get(&account_id);

        // If there is some set of active lease ids, process that ids set
        let active_lease_ids =
            if let Some(active_lease_ids_per_owner_set) = active_lease_ids_per_owner_set {
                active_lease_ids_per_owner_set
            } else {
                // If there is no active leases for the user, return an empty vector.
                return vec![];
            };

        // Get starting index, default to 0
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            (active_lease_ids.len() as u128) >= start_index,
            "Out of bounds. Please use a smaller from_index."
        );

        // Iterate through the keys vector
        active_lease_ids
            .iter()
            .skip(start_index as usize)
            .take(limit.unwrap_or(10) as usize)
            .map(|active_lease_id| {
                self.nft_token(self.lease_id_to_lease_token_id(&active_lease_id))
                    .unwrap()
            })
            .collect()
    }
}
