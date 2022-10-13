use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{ext_contract, AccountId};

// // Interface of this contract, for call backs - place holder
// #[ext_contract(ext_self)]
// pub trait Callbacks {
// }

// NFT interface, for cross-contract calls
#[ext_contract(ext_nft)]
pub trait Nft {
    // cross-contract call
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    );

    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        memo: Option<String>,
        msg: String,
    );
}
