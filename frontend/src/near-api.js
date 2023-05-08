import "near-api-js/dist/near-api-js.min.js";
const { connect, Contract, keyStores, WalletConnection } = window.nearApi;
import { getConfig } from "./near-config";
import { initFtContract } from "./FtContract";

export const nearConfig = getConfig(import.meta.env.MODE || "development");

export function signOutNearWallet() {
  window.walletConnection.signOut();
  // reload page
  window.location.replace(window.location.origin + window.location.pathname);
}

export function signInWithNearWallet() {
  // Allow the current app to make calls to the specified contract on the
  // user's behalf.
  // This works by creating a new access key for the user's account and storing
  // the private key in localStorage.
  window.walletConnection.requestSignIn(nearConfig.contractName);
}

// Initialize contract & set global variables
export async function initContract() {
  // Initialize connection to the NEAR testnet
  const near = await connect(
    Object.assign(
      { deps: { keyStore: new keyStores.BrowserLocalStorageKeyStore() } },
      nearConfig
    )
  );

  // Initializing Wallet based Account. It can work with NEAR testnet wallet that
  // is hosted at https://wallet.testnet.near.org
  window.walletConnection = new WalletConnection(near);

  // Getting the Account ID. If still unauthorized, it's just empty string
  window.accountId = window.walletConnection.getAccountId();

  // Initializing our contract APIs by contract name and configuration
  window.contract = await new Contract(
    window.walletConnection.account(),
    nearConfig.contractName,
    {
      viewMethods: [
        "list_listings_by_nft_contract_id",
        "list_allowed_nft_contract_ids",
        "list_allowed_ft_contract_ids",
        "get_listing_by_id",
        "get_rental_contract_id"
      ],
      changeMethods: [],
    }
  );

  // Initializing the rental contract.
  window.rentalContractId = await getRentalContractId();
  window.rentalContract = await new Contract(
    window.walletConnection.account(),
    window.rentalContractId,
    {
      viewMethods: ["leases_by_borrower", "leases_by_owner", "lease_by_contract_and_token"],
      changeMethods: ["claim_back"],
    });
}

export async function getRentalContractId() {
  return await window.contract.get_rental_contract_id();
}


export async function getAllowedFTs() {
  const ftAddrs = await window.contract.list_allowed_ft_contract_ids({});
  const fts = await Promise.all(ftAddrs.map(async addr => {
    const contract = await initFtContract(addr);
    const ftMetadata = await contract.ft_metadata({});
    return { address: addr, ...ftMetadata };
  }));
  return fts;
}

export async function acceptLease(leaseId, rent) {
  let response = await window.contract.lending_accept({
    args: {
      lease_id: leaseId,
    },
    gas: "300000000000000",
    amount: (BigInt(rent) + BigInt(1e18)).toString(),
  });
  return response;
}

export async function listingsByNftContractId(nftContractId) {
  const listings = await window.contract.list_listings_by_nft_contract_id({
    nft_contract_id: nftContractId,
  });
  return listings;
}

export async function listAllowedNftContractIds(nftContractId) {
  return await window.contract.list_allowed_nft_contract_ids({})
}

export async function listingByContractIdAndTokenId(nftContractId, tokenId) {
  const listing = await window.contract.get_listing_by_id({
    listing_id: [nftContractId, tokenId],
  });
  return listing;
}

export async function myLendings() {
  return await window.rentalContract.leases_by_owner({
    account_id: window.accountId,
  });
}

export async function myBorrowings() {
  return await window.rentalContract.leases_by_borrower({
    account_id: window.accountId,
  });
}

export async function leaseByContractIdAndTokenId(nftContractId, tokenId) {
  return await window.rentalContract.lease_by_contract_and_token({
    contract_id: nftContractId,
    token_id: tokenId,
  });
}

export async function claimBack(leaseId) {
  let response = await window.rentalContract.claim_back({
    args: {
      lease_id: leaseId,
    },
    gas: "300000000000000",
    amount: 1,
  });
  return response;
}