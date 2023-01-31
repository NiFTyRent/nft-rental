(WIP) NEAR NFT Rental Standard Proposal

## Rental Proxy

A rental proxy is a smart contrac serves as a middle agent to coordinate the NFT renting activities.

There may exist multiple rental proxies on the market, and the NFT utility providers has the ability (if they want) to choose which rental proxies are supported for their services. For example, they can ignore the rental proxies who don't pay the royalty split properly.

During the period of renting, the NFT will be kept in the custody of the rental proxy contract. Rental proxies should provide mechanisms for depositing and claiming back the NFTs.

A rental proxy needs to implement the following interface:

```rust
/// Returns the current legit borrower info
fn get_borrower(contract_addr: AccountId, token_id: TokenId) -> AccountId
```



## Look Up Flow

1. A view-only request send to the NFT contract's to look up the token owner. This will return the account of the rental proxy contract which is in use.
2. If the returned address is regocnised as a rental proxy in the allowlist, then send another view-only request to it, invoking the `get_borrower` function. This will return the current borrower, i.e. the current legit user of the NFT.



## Benefits

### Decoupled NFT core logic and the rental logic

The core logic of an NFT shouldn't change frequenly, if ever. Because it's about the permenent ownership. It need to be very rigid to gain the trust.

However, the rental logic can be and should be innovated to suit people's needs. And there should be allowed to have more than one rental offers for a same NFT, so that people could the best fit for their needs.

### Backward compatibility 

This approach requires no change to the NFT contracts. So it will support every NFT on the market automatically.

### Provides the utility provider enough control

NFT utility providers have the choice to block certain bad player among all rental proxies, for example the one who don't respect the agreed rental royalty split, or abues the rental system.
