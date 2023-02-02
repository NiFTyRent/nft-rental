use crate::*;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata, NFT_METADATA_SPEC,
}; //todo(syu): check is assert_valid() will break things in NFTContractMetadata and TokenMetadata

//The Json token to be returned for view calls.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct JsonToken {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: TokenMetadata,
}

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for Contract {
    // contract metatdata will be hardcoded for now
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
