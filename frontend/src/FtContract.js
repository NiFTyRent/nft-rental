
const { Contract } = window.nearApi;

export async function initFtContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["ft_metadata", "ft_balance_of"],
    changeMethods: ["ft_transfer_call"],
  });
}

export async function toNormalisedAmount(contract, amount) {
  const metadata = await contract.ft_metadata();
  const decimals = metadata.decimals;

  const scale = BigInt(10) ** BigInt(decimals - 3);
  const normalised = BigInt(Math.round(amount * 1000)) * scale;
  return normalised.toString();
}

export async function fromNormalisedAmount(contract, amount) {
  const metadata = await contract.ft_metadata();
  const decimals = metadata.decimals;

  const scale = BigInt(10) ** BigInt(decimals - 3);
  const normalised = BigInt(amount) / scale;
  return Number(normalised) / 1000;
}
