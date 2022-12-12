const { connect, Contract, keyStores, WalletConnection } = window.nearApi;
import { nearConfig } from "./near-api";

// Initialize contract & set global variables
export async function initContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["nft_tokens_for_owner", "nft_token"],
    changeMethods: ["nft_approve"],
  });
}

export async function nftTokensForOwner(contract, accountId) {
  if (accountId == "") return [];
  let tokens = await contract.nft_tokens_for_owner({
    account_id: window.accountId,
  });
  return tokens;
}

export async function getToken(contract, tokenId) {
  if (tokenId == "") return null;
  let token = await contract.nft_token({
    token_id: tokenId,
  });
  return token;
}

export async function newLease(
  contract,
  tokenId,
  borrower,
  expiration,
  amountNear
) {
  let YACTO = BigInt("1000000000000000000000000");
  let amountYacto = BigInt(amountNear) * YACTO;
  if (tokenId == "") return [];
  let message = JSON.stringify({
    contract_addr: contract.contractId,
    token_id: tokenId,
    borrower: borrower,
    expiration: expiration,
    price: amountYacto.toString(),
  });
  let tokens = await contract.nft_approve({
    args: {
      token_id: tokenId,
      account_id: nearConfig.contractName,
      msg: message,
    },
    gas: "300000000000000",
    amount: "1000000000000000000000",
  });
  return tokens;
}
