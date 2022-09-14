const { connect, Contract, keyStores, WalletConnection } = window.nearApi;
import { nearConfig } from "./near-api";

// Initialize contract & set global variables
export async function initContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["get_borrower"],
    changeMethods: [],
  });
}

export async function getBorrower(contract, nftContractId, tokenId) {
  let res = await contract.get_borrower({
    contract_id: nftContractId,
    token_id: tokenId,
  });
  return res;
}
