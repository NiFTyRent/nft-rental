use crate::*;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, NFT_METADATA_SPEC,
};

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    // Contract metatdata will be hardcoded for now
    fn nft_metadata(&self) -> NFTContractMetadata {
        NFTContractMetadata {
            spec: NFT_METADATA_SPEC.to_string(),
            name: "NiFTyRent Lease Ownership Token".to_string(),
            symbol: "LEASE".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
    }
}
