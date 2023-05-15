// This is a script to set the allowed FT contracts for a deployed testnet rental and marketplace contracts.
//
// expose RENTAL_CONTRACT=dev-123....
// expose MARKETPLACE_CONTRACT=dev-123....
// expose OWNER=someone.testnet
// Usage: near repl -s ./scripts/set_allowed_fts.js --accountId $OWNER

const FT_ADDRS = [
  "wrap.testnet",
  "usdt.fakes.testnet",
  "usdc.fakes.testnet",
]


module.exports.main = async function main({ account, near, nearAPI, argv }) {
  async function register_ft_deposit(ftAddrs, accountId) {
    const contract = new nearAPI.Contract(
      account,
      ftAddrs,
      {
        viewMethods: [],
        changeMethods: ["storage_deposit"],
      });

    await contract.storage_deposit({
      args: { account_id: accountId, registration_only: true },
      amount: "100000000000000000000000"
    }) // 0.1 NEAR
  }

  const rental_contract = new nearAPI.Contract(
    account,
    process.env.RENTAL_CONTRACT,
    {
      viewMethods: ["get_allowed_ft_contract_addrs"],
      changeMethods: ["set_allowed_ft_contract_addrs"],
    });
  const marketplace_contract = new nearAPI.Contract(
    account,
    process.env.MARKETPLACE_CONTRACT,
    {
      viewMethods: ["list_allowed_ft_contract_ids"],
      changeMethods: ["add_allowed_ft_contract_ids"]
    });

  console.log("Rental contract FTs before change:", await rental_contract.get_allowed_ft_contract_addrs());
  await Promise.all(FT_ADDRS.map(addr => { register_ft_deposit(addr, process.env.RENTAL_CONTRACT) }))
  await rental_contract.set_allowed_ft_contract_addrs({ args: { addrs: FT_ADDRS } });
  console.log("Rental contract FTs after change:", await rental_contract.get_allowed_ft_contract_addrs());

  console.log("Marketplace contract FTs before change:", await marketplace_contract.list_allowed_ft_contract_ids());
  await Promise.all(FT_ADDRS.map(addr => { register_ft_deposit(addr, process.env.MARKETPLACE_CONTRACT) }))
  await marketplace_contract.add_allowed_ft_contract_ids({ args: { ft_contract_ids: FT_ADDRS } });
  console.log("Marketplace contract FTs after change:", await marketplace_contract.list_allowed_ft_contract_ids());
};
