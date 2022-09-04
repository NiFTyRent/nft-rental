const { Contract } = nearAPI;

const contract = new Contract(account, account.accountId, {
  viewMethods: ["nft_tokens_for_owner"],
  changeMethods: ["new_default_meta", "migrate", "nft_mint"],
  sender: account,
});

await contract.migrate({ args: {} });
