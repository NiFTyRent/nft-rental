use near_contract_standards::non_fungible_token::{Token, metadata::TokenMetadata};

use crate::*;

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
        receiver_id: AccountId,
    ){
        todo!()
    }
}