import "near-api-js/dist/near-api-js.min.js";
const { connect, Contract, keyStores, WalletConnection } = window.nearApi;
import { getConfig } from "./near-config";

export const nearConfig = getConfig(import.meta.env.MODE || "development");

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
      viewMethods: ["nft_tokens_for_owner", "top_rank"],
      changeMethods: ["nft_mint_2022"],
    }
  );
}

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

export async function nftMint2022(receiver_id) {
  let response = await window.contract.nft_mint_2022({
    args: { receiver_id: receiver_id },
    amount: "1000000000000000000000000",
  });
  console.log(response);
  return response;
}

export async function topRank() {
  let rank = await window.contract.top_rank({});
  return rank;
}

export async function myTokens() {
  let tokens = await window.contract.nft_tokens_for_owner({
    account_id: window.accountId,
  });
  return tokens;
}
