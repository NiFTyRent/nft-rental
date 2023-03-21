use crate::*;
use near_sdk::PromiseOrValue;

/// NFT interface, for cross-contract calls
#[ext_contract(ext_nft)]
pub trait NonFungibleToken {
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        msg: String,
        approval_id: Option<u64>,
        memo: Option<String>,
    );
}

#[ext_contract(ext_ft)]
pub trait FungibleToken {
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
}

/// External interface of this marketplace contract
#[ext_contract(ext_self)]
trait ExtSelf {
    fn transfer_rent_after_nft_transfer(
        &mut self,
        ft_contract_id: AccountId,
        amount: U128,
        memo: Option<String>,
    ) -> PromiseOrValue<U128>;
}
