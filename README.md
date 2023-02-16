# NiftyRent

## Quick Start

If you haven't installed dependencies during setup:

    yarn deps-install

To deploy dev contract on testnet

    yarn deploy

To start dev server

    yarn start


## Allowed FT contract addresses

For now, the contract only allow a limited number of FTs as the rent payment currency options.

To check the current list:

    near call $CONTRACT_NAME get_allowed_ft_contract_addrs "" --accountId $ACCOUNT_ID


To set the list (for the testnet for example):

    near call $CONTRACT_NAME set_allowed_ft_contract_addrs '{"addrs": ["wrap.testnet", "usdc.fakes.testnet"]}' --accountId $ACCOUNT_ID

Bear in mind that you need to make sure:

1. `$ACCOUNT_ID` is the owner of the contract. (Only owner can change the list.)
2. the contract itself have been registered in the FT contract for the storage deposit. For example, you can: `near call usdc.fakes.testnet storage_deposit "{\"account_id\": \"$CONTRACT_NAME\", \"registration_only\": true}" --accountId $ACCOUNT_ID --amount 0.1`

Once updated, the UI should automatically pick up the new list of allowed FTs.
