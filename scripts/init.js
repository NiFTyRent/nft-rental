const { Contract } = nearAPI;

const contract = new Contract(account, account.accountId, {
  viewMethods: ["nft_tokens_for_owner"],
  changeMethods: ["new_default_meta", "nft_mint"],
  sender: account,
});

await contract.new_default_meta({ args: { owner_id: account.accountId } });
