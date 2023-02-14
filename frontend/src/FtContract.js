const { Contract } = window.nearApi;

export async function initFtContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["ft_metadata", "ft_balance_of"],
    changeMethods: ["ft_transfer_call"],
  });
}
