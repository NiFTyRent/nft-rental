
const { Contract } = window.nearApi;

export async function initFtContract(contractName) {
  return await new Contract(window.walletConnection.account(), contractName, {
    viewMethods: ["ft_metadata", "ft_balance_of"],
    changeMethods: ["ft_transfer_call"],
  });
}

export function toNormalisedAmount(contractId, amount) {
  const metadata = window.CURRENCY_OPTIONS.find((m) => m.address === contractId);
  const decimals = metadata.decimals;

  const scale = BigInt(10) ** BigInt(decimals - 3);
  const normalised = BigInt(Math.round(amount * 1000)) * scale;
  return normalised.toString();
}

export function fromNormalisedAmount(contractId, amount) {
  const metadata = window.CURRENCY_OPTIONS.find((m) => m.address === contractId);
  const decimals = metadata.decimals;

  const scale = BigInt(10) ** BigInt(decimals - 3);
  const normalised = BigInt(amount) / scale;
  return Number(normalised) / 1000;
}

export function ftSymbol(contractId) {
  const metadata = window.CURRENCY_OPTIONS.find((m) => m.address === contractId);
  return metadata.symbol;
}
