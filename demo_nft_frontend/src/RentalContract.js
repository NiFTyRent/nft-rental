const { Contract } = window.nearApi;

// Initialize contract & set global variables
export async function initContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["get_borrower_by_contract_and_token"],
    changeMethods: [],
  });
}

export async function getBorrower(contract, nftContractId, tokenId) {
  let res = await contract.get_borrower_by_contract_and_token({
    contract_id: nftContractId,
    token_id: tokenId,
  });
  return res;
}
