use near_sdk::PromiseOrValue;

use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct LeaseAcceptanceJson {
    nft_contract_id: AccountId,
    nft_token_id: AccountId,
}

/// The trait for receiving FT payment
pub trait FungibleTokenReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}
#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Function that initiates the transaction of activating a listed lease.
    #[payable]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let lease_acceptance_json: LeaseAcceptanceJson =
            near_sdk::serde_json::from_str(&msg).expect("Invalid lease listing");

        let listing_id: ListingId = (
            lease_acceptance_json.nft_contract_id,
            lease_acceptance_json.nft_token_id,
        );

        let listing: Listing = self.listings.get(&listing_id).unwrap();

        assert_eq!(
            listing.ft_contract_id,
            env::predecessor_account_id(),
            "Wrong FT contract id!"
        );
        assert_eq!(
            amount.0, listing.price.0,
            "Transferred amount doesn't match the asked rent!"
        );

        // Transfer both the to be rented NFT and the rent payment (FT) to the rental contract.
        // And the rental contract will active the lease.
        // When it returns successfully, remove the listing.
        todo!();
    }
}
