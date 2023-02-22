
# Introduction
Both unit tests and integrations tests are included for the project.
## Prerequisite
Follow ../README.md to set up the environment

# Unit Test
Unit Tests are located in the contract file ./contract/src/lib.rs, inside the tests module.

## How to run unit tests
- move to project root directory. e.g. ./nft-rental
- `yarn test:unit`

# Integration Test
Integration test are based in a different directory ./integration-tests
## How to run test
- move to the ./nft-rental/integration-tests
- Run `yarn test:integration`

## Test plan

- `test_claim_back_with_payout_success`: the lender, Alice, lends the testing NFT token to the borrower, Bob. This test will verify the ownership of the token is transferred back to Alice after being claimed back. The test will also verify the payout splits are made correctly as defined in the token.
- `test_claim_back_without_payout_success`: similar to `test_claim_back_with_payout_success` but with an NFT which doesn't support payout. In this case all the rent should go to Alice.
- `test_accept_leases_already_lent`: This test verifies that the call will pass for the first time when lender accepts the lease but should faile if borrowers accepts the same lease for multiple times.
- `test_accept_lease_fails_already_transferred`: This test verifies that the call should fail if the token has been transferred before the borrowers accepts the lease.

Inline comment and test output have also been added. Please refer the code.
