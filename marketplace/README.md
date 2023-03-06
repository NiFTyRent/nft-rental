NiFTyRent Marketplace contract
=================================

A marketplace for users to lend and rent utility NFT assets.

The actual rental logic is delegated to the core rental contract, for sake of responsibility separation.

``` mermaid
sequenceDiagram
    actor Lender
    actor Borrower
    participant Marketplace
    participant Core
    participant Game
    Lender->>Marketplace: Create lease offer
    Borrower->>Marketplace: Take lease offer (by tranfer the rent)
    Marketplace->>Core: Transfer NFT, transfer rent, and create an active lease
    Game ->> Core: check current user
    Lender ->> Core: claim back the NFT and rent
```

