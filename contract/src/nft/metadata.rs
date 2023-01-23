use near_contract_standards::non_fungible_token::metadata::NFTContractMetadata;


use crate::*;
pub trait NonFungibleTokenMetadata {
    fn nft_metadata(&self) -> NFTContractMetadata;
}

impl NonFungibleTokenMetadata for Contract {
    fn nft_metadata(&self) -> NFTContractMetadata {
        todo!()
    }
}