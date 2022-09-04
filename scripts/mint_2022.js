const { Contract } = nearAPI;

const contract = new Contract(account, account.accountId, {
  viewMethods: ["nft_tokens_for_owner"],
  changeMethods: ["new_default_meta", "nft_mint_2022"],
  sender: account,
});

await contract.nft_mint_2022({
  args: {
    receiver_id: "libo.testnet",
  },
  amount: "168310000000000000000000",
});
