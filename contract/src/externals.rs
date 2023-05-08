use crate::*;

/// Interface of this contract
// TODO(libo): explicitly implement this trait.
#[ext_contract(ext_self)]
trait ExtSelf {
    fn activate_lease(&mut self, lease_id: LeaseId) -> PromiseOrValue<U128>;
    fn resolve_claim_back(&mut self, lease_id: LeaseId) -> Promise;
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
}

#[ext_contract(ext_ft_core)]
pub trait FungibleTokenCore {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
}
