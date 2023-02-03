use crate::*;

use near_contract_standards::non_fungible_token::{
    metadata::TokenMetadata,
    Token,};
pub use near_contract_standards::non_fungible_token::core::NonFungibleTokenCore;
use near_sdk::{assert_one_yocto, PromiseOrValue, PromiseResult};


const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
const GAS_FOR_NFT_ON_TRANSFER: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);

#[ext_contract(ext_nft_receiver)]
trait NonFungibleTokenReceiver {
    /// Method on the receiver contract that is called via XCC when nft_transfer_call is called
    /// Returns `true` if the token should be returned back to the sender.
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> Promise;
}

#[ext_contract(ext_self)]
trait NonFungibleTokenResolver {
    /// This method resolves the promise returned from the XCC to the receiver contract,
    /// as part of the nft_transfer_call method
    fn nft_resolve_transfer(
        &mut self,
        owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
    ) -> bool;
}

impl NonFungibleTokenCore for Contract {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>
    ){
        // Security assurance, on full access
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            memo,
        );
    }

    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool> {
        assert_one_yocto();
        let sender_id = env::predecessor_account_id();
        let previous_token = self.internal_transfer(
            &sender_id,
            &receiver_id,
            &token_id,
            memo,
        );

        ext_nft_receiver::ext(receiver_id.clone())
            .with_static_gas(GAS_FOR_NFT_ON_TRANSFER)
            .nft_on_transfer(
                sender_id,
                previous_token.owner_id.clone(),
                token_id.clone(),
                msg,
            )
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .nft_resolve_transfer(previous_token.owner_id, receiver_id, token_id),
            )
            .into()
    }

    /// Returns the token info with a given token_id. Info are assembled on the fly
    fn nft_token(&self, token_id: TokenId) -> Option<Token> {
        if self.active_lease_ids.contains(&token_id) {

            // Get the lease condition to assemble token info and token metadata
            let lease_condition = self.lease_map.get(&token_id).unwrap();

            // Generate token metadata on the fly. Hard coded for now
            let token_metadata = TokenMetadata{
                title: Some(format!("NiftyRent Lease Ownership Token: {}", &token_id)), 
                description: Some(
                    format!("
                    This is a token representing the ownership the NFT under the NiFTyRent lease: {lease_id}\n
                    The under-leased NFT is {leased_token_id} at {contract_id}", 
                    lease_id=&token_id,
                    leased_token_id=&lease_condition.token_id, 
                    contract_id=&lease_condition.contract_addr
                )),
                media: None,    // TODO(syu): add the media link         
                media_hash: None,
                copies: None,
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: None,
                reference_hash: None,
            };

            //Return the token object with assembed info
            Some(Token {
                token_id,
                owner_id: lease_condition.lender_id,
                metadata: Some(token_metadata),
                approved_account_ids: None,     // TODO(syu): Add support for Approval
            })
        } else {
            //if there wasn't any token_id in tokens_by_id, return None
            None
        }
    }
}

impl NonFungibleTokenResolver for Contract {
    /// resolves XCC result from receiver's nft_on_transfer
    /// returns true if the token was successfully transferred to the receiver_id
    fn nft_resolve_transfer(
        &mut self,
        previouse_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: TokenId,
    ) -> bool {
        // Check whether the token should be returned to previous owner
        let should_revert = match env::promise_result(0) {
            PromiseResult::NotReady => env::abort(),
            PromiseResult::Successful(value) => {
                if let Ok(true_or_false) = near_sdk::serde_json::from_slice::<bool>(&value) {
                    true_or_false
                } else {
                    true
                }
            }
            PromiseResult::Failed => true,
        };

        // If the XCC indicated no revert, return early
        if !should_revert {
            return true;
        }

        // Otherwise, try to revert this transfer and return the token to the previous owner
        if let Some(lease_condition) = self.lease_map.get(&token_id) {
            // Check that the receiver didn't transfer the token away or burned it
            if lease_condition.lender_id != receiver_id {
                // The token is no longer owned by the receiver. Can't return it
                return true;
            }
        } else {
            // no token_id record. The token doesn't exist any more, or got burned
            return true;
        }

        // At this stage, we can safely revert the transfer
        log!(
            "Return LEASE Token {} from @{} to @{}",
            token_id,
            receiver_id,
            previouse_owner_id
        );
        
        self.internal_update_active_lease_lender(&receiver_id, &previouse_owner_id, &token_id);

        return false;
    }
}