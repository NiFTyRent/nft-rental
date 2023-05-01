import React from "react";
import { nearConfig, listingByContractIdAndTokenId } from "./near-api";
import { fromNormalisedAmount, ftSymbol, initFtContract } from "./FtContract";
import { NftInfo } from "./NftInfo";
import { useParams } from "react-router-dom";
import { dateTimeString } from "./Utils";

export default function ListingAcceptPage() {
  let { contractId, tokenId } = useParams()
  const [listing, setListing] = React.useState(null);

  React.useEffect(() => {
    async function fetchListing() {
      let listing = await listingByContractIdAndTokenId(contractId, tokenId);
      setListing((_) => listing);
    }
    fetchListing();
  }, [contractId, tokenId]);

  let onSubmit = async () => {
    if (!listing) return;
    const ftContract = await initFtContract(listing.ft_contract_id);
    const amount = BigInt(listing.price).toString();
    return await ftContract.ft_transfer_call({
      args: {
        receiver_id: nearConfig.contractName,
        amount: amount,
        msg: JSON.stringify({ listing_id: [contractId, tokenId] })
      },
      gas: "300000000000000",
      amount: "1",
    })
  };


  return listing ? (
    <>
      <div className="py-6">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <h1 className="text-2xl mb-8 font-semibold text-gray-900">
            Rent NFT
          </h1>
        </div>
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <div className="space-y-8 divide-y divide-gray-200">
            <div className="flex flex-col space-y-8 divide-y divide-gray-200">
              <div className="space-y-6">
                <h3 className="text-lg font-medium leading-6 text-gray-900">
                  NFT Info
                </h3>

                <NftInfo contractId={listing.nft_contract_id} tokenId={listing.nft_token_id} />

                <h3 className="text-lg font-medium leading-6 text-gray-900">
                  Lease Info
                </h3>

                <div className="space-y-6 sm:space-y-4">
                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Lease Start Time
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {dateTimeString(listing.lease_start_ts_nano)}
                    </div>
                  </div>
                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Lease End Time
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {dateTimeString(listing.lease_end_ts_nano)}
                    </div>
                  </div>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Rent
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {fromNormalisedAmount(listing.ft_contract_id, listing.price)} {ftSymbol(listing.ft_contract_id)}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="pt-5">
                <div className="flex justify-end">
                  <a
                    className="rounded-md border border-gray-300 bg-white py-2 px-4 text-sm font-medium text-gray-700 shadow-sm hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                    href="/"
                  >
                    Cancel
                  </a>
                  <button
                    className="ml-3 inline-flex justify-center rounded-md border border-transparent bg-indigo-600 py-2 px-4 text-sm font-medium text-white shadow-sm hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2"
                    onClick={(_) => onSubmit()}
                  >
                    Accept & Pay
                  </button>
                </div>
            </div>
          </div>
        </div>
      </div >
    </>
  ) : (
    "Loading"
  );
}
