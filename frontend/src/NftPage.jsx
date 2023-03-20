import React from "react";
import { useParams } from "react-router-dom";
import { NftInfo } from "./NftInfo";


export default function AcceptBorrowingPage() {
  let { contractId, tokenId } = useParams();
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
              href={"/app/nfts/" + contractId + "/" + tokenId + "/lend"}
              className="primary-btn"
            >
              Lend
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
