// List all state keys of a given contract on testnet.
//
// Useful for clear the contract state completely when it's needed.
//
// See more: https://gist.github.com/ilyar/19bdc04d1aa09ae0fc84eb4297df1a1d
// Example use:
//
// expose CONTRACT_NAME=dev-123....
// expose OWNER_ID=someone.testnet
// near call $CONTRACT_NAME clean --base64 "$(node scripts/view_state_keys.js | base64)" --accountId $CONTRACT_NAME --gas 300000000000000


const nearAPI = require('near-api-js')
const { connect, keyStores } = nearAPI
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(__dirname);
const config = {
  keyStore,
  networkId: 'testnet',
  nodeUrl: 'https://rpc.testnet.near.org',
  walletUrl: 'https://wallet.testnet.near.org',
  helperUrl: 'https://helper.testnet.near.org',
  explorerUrl: 'https://explorer.testnet.near.org',
}

async function main () {
  const near = await connect(config)
  const response = await near.connection.provider.query({
    request_type: 'view_state',
    finality: 'final',
    account_id: process.env.CONTRACT_NAME,
    prefix_base64: '',
  })
  console.log(JSON.stringify({
    // TODO add calc size of data for limit burning 200TGas for one call on contract
    keys: response.values.map(it => it.key),
    owner_id: process.env.OWNER_ID
  }))
}

main().catch(reason => {
  console.error(reason)
})
