const { Contract } = window.nearApi;
import { nearConfig } from "./near-api";
import { initFtContract } from "./FtContract"

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
  ftAddress,
  price,
) {
  if (tokenId == "") return;
  const ftContract = await initFtContract(ftAddress);
  const ftMetadata = await ftContract.ft_metadata();
  const ftDecimals = ftMetadata.decimals;

  const scale = 10n ** BigInt(ftDecimals - 3);
  const priceNormalised = BigInt(Math.round(price * 1000)) * scale;
  const message = JSON.stringify({
    contract_addr: contract.contractId,
    token_id: tokenId,
    borrower_id: borrower,
    expiration: expiration,
    ft_contract_addr: ftAddress,
    price: priceNormalised.toString(),
  });
  return await contract.nft_approve({
    args: {
      token_id: tokenId,
      account_id: nearConfig.contractName,
      msg: message,
    },
    gas: "300000000000000",
    amount: "1000000000000000000000",
  });
}
