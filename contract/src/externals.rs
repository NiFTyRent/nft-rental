use crate::*;

/// Interface of this contract
#[ext_contract(ext_self)]
trait ExtSelf {
    fn activate_lease(&mut self, lease_id: LeaseId) -> Promise;
    fn resolve_claim_back(&mut self, lease_id: LeaseId) -> Promise;
    fn create_lease_with_payout(
        &mut self,
        contract_id: AccountId,
        token_id: TokenId,
        owner_id: AccountId,
        borrower_id: AccountId,
        expiration: u64,
        price: u128,
        approval_id: u64,
    ) -> Promise;
}

/// NFT interface, for cross-contract calls
/// For details, refer to NEP-171
#[ext_contract(ext_nft)]
pub trait Nft {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    );

    /// Payout Support
    /// See https://nomicon.io/Standards/Tokens/NonFungibleToken/Payout
    fn nft_transfer_payout(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
        balance: U128,
        max_len_payout: Option<u32>,
    );

    fn nft_payout(self, token_id: String, balance: U128, max_len_payout: Option<u32>) -> Payout;
}
