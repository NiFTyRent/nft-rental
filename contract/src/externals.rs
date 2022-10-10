use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{ext_contract, AccountId, Gas};

pub const TGAS: u64 = 1_000_000_000_000;
pub const XCC_GAS: Gas = Gas(5 * TGAS); // cross contract gas

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
