use crate::{*, nft::core::NonFungibleTokenCore};
use near_sdk::json_types::U128;

/// NEP-181 Enumeration
/// Offers methods helpful in determining account ownership of NFTs
/// and provides a way to page through NFTs per owner, determine total supply, etc.
pub trait NonFungibleTokenEnumeration {
    /// Returns the total supply of non-fungible tokens as a string representing an
    /// unsigned 128-bit integer to avoid JSON number limit of 2^53.
    fn nft_total_supply(&mut self) -> U128;

    /// Get a list of all tokens
    ///
    /// Arguments:
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns an array of Token objects, as described in Core standard
    fn nft_tokens(
        &mut self,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // default: unlimited (could fail due to gas limit)
    ) -> Vec<Token>;

    /// Get the number of tokens owned by a given account
    fn nft_supply_for_owner(&mut self, account_id: AccountId) -> U128;

    /// Get list of all tokens owned by a given account
    ///
    /// Arguments:
    /// * `account_id`: a valid NEAR account
    /// * `from_index`: a string representing an unsigned 128-bit integer,
    ///    representing the starting index of tokens to return
    /// * `limit`: the maximum number of tokens to return
    ///
    /// Returns a paginated list of all tokens owned by this account
    fn nft_tokens_for_owner(
        &mut self,
        account_id: AccountId,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // default: unlimited (could fail due to gas limit)
    ) -> Vec<Token>;
}

impl NonFungibleTokenEnumeration for Contract {
    fn nft_total_supply(&mut self) -> U128 {
        U128(self.token_metadata_by_id.len() as u128)
    }

    /// Query for all nft tokens on the contract. Using Pagination
    fn nft_tokens(
        &mut self,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // default: unlimited (could fail due to gas limit)
    ) -> Vec<Token> {
        // Get starting index, default to 0
        let start_index: u128 = from_index.map(From::from).unwrap_or_default();
        require!(
            (self.token_metadata_by_id.len() as u128) >= start_index,
            "Out of bounds, please use a smaller from_index."
        );

        // sainity check on limit
        let limit = limit.map(|v| v as usize).unwrap_or(usize::MAX);
        require!(limit != 0, "limit cannot be 0.");

        self.token_metadata_by_id
            .iter()
            .skip(start_index as usize)
            .take(limit)
            .map(|(token_id, _token_metadata)| self.nft_token(token_id.clone()).unwrap())
            .collect()
    }

    /// Get total NFT supply for a given account
    fn nft_supply_for_owner(&mut self, account_id: AccountId) -> U128 {
        let token_ids_set = self.token_ids_per_owner.get(&account_id);

        if let Some(token_ids_set) = token_ids_set {
            U128(token_ids_set.len() as u128)
        } else {
            U128(0)
        }
    }

    fn nft_tokens_for_owner(
        &mut self,
        account_id: AccountId,
        from_index: Option<U128>, // default: "0"
        limit: Option<u64>,       // default: unlimited (could fail due to gas limit)
    ) -> Vec<Token> {
        todo!()
    }
}
