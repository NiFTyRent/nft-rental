# NiftyRent

## Quick Start

If you haven't installed dependencies during setup:

    yarn deps-install

To deploy the rental dev contract on testnet

    yarn deploy

Initilise the rental dev contract

    export OWNER=<your test account id>
    export RENTAL_CONTRACT=`cat contract/neardev/dev-account`
    near call $RENTAL_CONTRACT new "{\"owner_id\": \"$OWNER\"}" --accountId $OWNER

Deploy the marketplace dev contract on testnet

    yarn deploy:marketplace

Initilise the marketplace dev contract

    export MARKETPLACE_CONTRACT=`cat marketplace/neardev/dev-account`
    near call $MARKETPLACE_CONTRACT new "{\"owner_id\": \"$OWNER\", \"treasury_id\": \"$OWNER\", \"rental_contract_id\": \"$RENTAL_CONTRACT\"}" --accountId $OWNER

Add and register allowed FTs for both contracts

    near repl -s ./scripts/set_allowed_fts.js --accountId $OWNER

Add allowed NFT contracts for the marketplace contract

    near call $MARKETPLACE_CONTRACT add_allowed_nft_contract_ids '{"nft_contract_ids": ["niftyrpg.mintspace2.testnet"]}' --accountId $OWNER
    // You can add more testing NFT contracts





To start dev server

    yarn start


## Allowed FT contract addresses

For now, the contract only allow a limited number of FTs as the rent payment currency options.

To check the current list:

    near call $CONTRACT_NAME get_allowed_ft_contract_addrs "" --accountId $ACCOUNT_ID


To set the list (for the testnet for example):

UPDATE: there is script automates the following: `./scripts/set_allowed_fts.js`

    near call $CONTRACT_NAME set_allowed_ft_contract_addrs '{"addrs": ["wrap.testnet", "usdc.fakes.testnet"]}' --accountId $ACCOUNT_ID

Bear in mind that you need to make sure:

1. `$ACCOUNT_ID` is the owner of the contract. (Only owner can change the list.)
2. the contract itself have been registered in the FT contract for the storage deposit. For example, you can: `near call usdc.fakes.testnet storage_deposit "{\"account_id\": \"$CONTRACT_NAME\", \"registration_only\": true}" --accountId $ACCOUNT_ID --amount 0.1`

Once updated, the UI should automatically pick up the new list of allowed FTs.
