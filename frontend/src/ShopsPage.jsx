import React from "react";
import { contractIdToName } from "./Utils";

export default function ShopPage() {
  const [shops, setShops] = React.useState([]);

  React.useEffect(() => {
    async function fetchContractIds() {
      const nftContractIds = await window.contract.list_allowed_nft_contract_ids({})
      setShops((_) => nftContractIds.map((nftContractId) => (
        { contractId: nftContractId, name: contractIdToName(nftContractId) }
      )))
    }
    fetchContractIds();
  }, []);

  return (
    <div className="px-4 py-4 sm:px-6 lg:px-8">
      <div className="sm:flex sm:items-center mb-8">
        <div className="sm:flex-auto">
          <h1 className="text-3xl font-semibold text-gray-900">Shops</h1>
        </div>
      </div>
      <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
        {
          // TODO(libo): filter out the Lease Ownership Token contract
          shops.map(({ name, contractId }) => {
            return <div key={contractId} className="border p-4 border-black rounded-md space-y-4">
              <p>{name}</p>
              {/* <span className="h-36 w-36 overflow-hidden  bg-gray-100">
                <img className="w-full" src={nft_info.media} />
                </span> */}
              <div className="flex flex-row justify-center space-x-2">
                <a href={"/app/shops/" + contractId + "/"}
                  className="btn flex-1 w-32 text-center"> Details </a>
              </div>
            </div>
          })
        }
      </div>
    </div>
  )
}