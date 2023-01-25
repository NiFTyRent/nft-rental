use crate::*;

 impl Contract {
    /// returns the total number of active leases
    /// useful for nft_total_supply() in IOU nft
    pub(crate) fn total_active_leases(&mut self) -> u128{
        todo!()
    }

    pub(crate) fn internal_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        token_id: &TokenId,
        memo: Option<String>
    ) -> Token {
        // 1. update token record to new owner
        // 2. update lease condition to reflect the lender change
        todo!()
    }

}