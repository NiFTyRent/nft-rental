use crate::*;

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
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
        memo: Option<String>,
    );
}
