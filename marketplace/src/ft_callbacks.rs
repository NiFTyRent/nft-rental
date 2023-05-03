use near_sdk::PromiseOrValue;

use crate::externals::*;
use crate::*;

/// Message to be passed in by borrower. The listing_id is available in the dApp's front end
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct ListingAcceptanceJson {
    listing_id: ListingId,
}

/// The trait for receiving rent payment and trigering listing acceptance.
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
 * 1. Borrower(Sender) calls `ft_transfer_call` on FT contract.
 * 2. FT contract transfers `amount` tokens from Borrower to Marketplace(reciever).
 * 3. FT contract calls `ft_on_transfer` on Marketplace contract.
 * 4.1 Marketplace contract makes XCC (nft_transfer_call) to transfer the leasing NFT to Core contract.
 * 4.2 Marketplace contract makes XCC (ft_transfer) to transfer rent to Core contract.
 * 5. Marketplace contract resolves the promise returned from Core and returns Promise accordingly.
*/
#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    // TODO(syu): check if ft transfer can be reverted back to borrower, if transaction failed.
    /// Function that initiates the transaction of activating a listed lease.
    #[payable]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // Enforce cross contract call
        let ft_contract_id = env::predecessor_account_id();
        assert_ne!(
            ft_contract_id,
            env::current_account_id(),
            "ft_on_transfer should only be called via XCC"
        );

        // Get the target listing ID
        let listing_acceptance_json: ListingAcceptanceJson =
            near_sdk::serde_json::from_str(&msg).expect("Invalid lease listing");

        let listing: Listing = self
            .listing_by_id
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
        // The Core rental contract will activate the lease.
        // When Core returns successfully, remove the listing in marketplace
        // 1. Marketplace transfers the NFT to Core contract
        //    1.1 Core contract will create the lease
        // 2. Marketplace transfers rent to Core contract
        // 3. Marketplace reolves the result from the above two steps and returns accordingly

        // msg to be passed in nft_transfer_call for a lease creation
        let msg_lease_json = json!({
            "nft_contract_id": listing.nft_contract_id.clone(),
            "nft_token_id": listing.nft_token_id.clone(),
            "lender_id": listing.owner_id.clone(),
            "borrower_id": sender_id.clone(),
            "ft_contract_addr": listing.ft_contract_id.clone(),
            "price": listing.price.clone(),
            "start_ts_nano": listing.lease_start_ts_nano.clone(),
            "end_ts_nano": listing.lease_end_ts_nano.clone(),
            "nft_payout":listing.payout.clone().unwrap(),
        })
        .to_string();

        // log nft transfer
        env::log_str(
            &json!({
                "type": "[INFO] NiFTyRent Marketplace: transfer leasing nft.",
                "params": {
                    "nft_contract_id": listing.nft_contract_id.clone(),
                    "nft_token_id": listing.nft_token_id.clone(),
                    "lender": listing.owner_id.clone(),
                    "borrower": sender_id.clone(),
                    "nft_payout": listing.payout.clone().unwrap(),
                }
            })
            .to_string(),
        );

        // Transfer the leasing nft to Core contract
        ext_nft::ext(listing.nft_contract_id.clone())
            .with_static_gas(Gas(10 * TGAS))
            .with_attached_deposit(1)
            .nft_transfer_call(
                self.rental_contract_id.clone(),   // receiver_id
                listing.nft_token_id.clone(),      // nft_token_id
                msg_lease_json,                    // msg
                Some(listing.approval_id.clone()), // approval_id
                None,                              // memo
            )
            .then(
                // Trasnfer the rent to Core contract, after resolving the returned promise
                // listing will also be removed when both transfers succeeded
                ext_self::ext(env::current_account_id())
                    .with_static_gas(Gas(10 * TGAS))
                    .transfer_rent_after_nft_transfer(
                        listing.ft_contract_id.clone(), // ft_contract_id
                        listing.price.clone(),          // amount
                        None,                           // memo
                        listing_acceptance_json.listing_id,
                    ),
            )
            .as_return()
            .into()
    }
}
