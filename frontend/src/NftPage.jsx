import React from "react";
import { useParams, useSearchParams } from "react-router-dom";
import { NftInfo } from "./NftInfo";


export default function AcceptBorrowingPage() {
  const { contractId } = useParams();
  const [searchParams, _setSearchParams] = useSearchParams();
  const tokenId = searchParams.get("tokenId")
  return (
    <>
      <div className="py-6">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8 space-y-8">
          <h1 className="text-2xl mb-8 font-semibold text-gray-900">
            NFT Details
          </h1>
          <NftInfo contractId={contractId} tokenId={tokenId} />
          <div className="pt-5 space-x-4">
            <a
              href={"/app/nfts/" + contractId + "/lend" + "?tokenId=" + tokenId}
            >
              <div className="primary-btn inline-block">
                Lend
              </div>
            </a>
            <button
              className="btn"
              onClick={(_) => history.back()}
            >
              Back
            </button>
          </div >
        </div >
      </div>
    </>
  );
}
