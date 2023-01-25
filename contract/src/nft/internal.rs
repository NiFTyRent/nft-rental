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
        todo!()
    }

 }