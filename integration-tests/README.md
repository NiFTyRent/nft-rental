# Integration Test

## prerequisite

Follow ../README.md to set up the environment

## How to run test

Run command `yarn test:integration` to run the integration test

## Test plan

- `test_claim_back_success`: the lender, Alice, lends the testing NFT token to the borrower, Bob. This test will verify the ownership of the token is transferred back to Alice after being claimed back.
- `test_accept_leases_already_lent` will verify the call will pass for the first time when lender accepts the lease but failed if borrowers accepts the same lease for multiple times.
- `test_accept_lease_fails_already_transferred` will verify the call will fail if the token has been transferred before the borrowers accept the lease.
