use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::{ext_contract, AccountId, Gas};

pub const TGAS: u64 = 1_000_000_000_000;
pub const XCC_GAS: Gas = Gas(5 * TGAS); // cross contract gas

// Interface of this contract, for call backs
#[ext_contract(this_contract)]
trait Callbacks {
    fn lending_accept_callback(&mut self) -> bool;
}

// NFT interface, for cross-contract calls
#[ext_contract(nft)]
trait Nft {
    // cross-contract call
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    );
}
