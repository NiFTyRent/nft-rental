use near_sdk::PromiseOrValue;

use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ListingAcceptanceJson {
    listing_id: String,
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

/**
 * This method will triger the acceptance of a listing. 
 * 1. Borrower(Sender) calls `ft_transfer_call` on FT contract
 * 2. FT contract transfers `amount` tokens from Borrower to Marketplace(reciever)
 * 3. FT contract calls `ft_on_transfer` on Marketplace contract
 * 4. Marketplace contract makes XCC (nft_transfer_call) to transfer the leasing NFT to Core contract
 * 5. Marketplace contract resolves the promise returned from Core ands return Promise accordingly
*/
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
        // enforce cross contract call
        let ft_contract_id = env::predecessor_account_id();
        assert_ne!(
            ft_contract_id,
            env::current_account_id(),
            "ft_on_transfer should only be called via XCC"
        );

        let listing_acceptance_json: ListingAcceptanceJson =
            near_sdk::serde_json::from_str(&msg).expect("Invalid lease listing");

        let listing: Listing = self
            .listings
            .get(&listing_acceptance_json.listing_id)
            .unwrap();

        assert_eq!(
            ft_contract_id.clone(),
            listing.ft_contract_id,
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
