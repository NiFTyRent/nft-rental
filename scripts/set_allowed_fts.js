// This is a script to set the allowed FT contracts for a deployed testnet rental contract.
//
// expose CONTRACT_NAME=dev-123....
// expose OWNER_ID=someone.testnet
// Usage: near repl -s ./scripts/set_allowed_fts.js --accountId $ACCOUNT_ID

const FT_ADDRS = [
  "wrap.testnet",
  "usdt.fakes.testnet",
  "usdc.fakes.testnet",
]


module.exports.main = async function main({account, near, nearAPI, argv}) {

  const contract = new nearAPI.Contract(
    account,
    process.env.CONTRACT_NAME,
    {
        viewMethods: ["get_allowed_ft_contract_addrs"],
        changeMethods: ["set_allowed_ft_contract_addrs"],
  });
  console.log("FTs before change:", await contract.get_allowed_ft_contract_addrs());

  async function register_ft_deposit(ftAddrs, accountId) {
    const contract = new nearAPI.Contract(
      account,
      ftAddrs,
      {
          viewMethods: [],
          changeMethods: ["storage_deposit"],
    });

    await contract.storage_deposit({
      args: {account_id: accountId, registration_only: true},
      amount: "100000000000000000000000"}) // 0.1 NEAR
  }

  await Promise.all(FT_ADDRS.map(addr => {
    register_ft_deposit(addr, process.env.CONTRACT_NAME)
  }))

  await contract.set_allowed_ft_contract_addrs({args: {addrs: FT_ADDRS}});

  console.log("FTs after change:", await contract.get_allowed_ft_contract_addrs());
};
