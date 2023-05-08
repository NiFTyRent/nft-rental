import React from "react";
import { useQuery, gql } from "@apollo/client";
import { leaseByContractIdAndTokenId } from "./near-api";

export function NftInfo({ contractId, tokenId }) {
  const GET_TOKEN = gql`
    query GetTokens($contract_id: String!, $token_id: String!) {
      mb_views_nft_tokens(where: {nft_contract_id: {_eq: $contract_id}, token_id: {_eq: $token_id}}, limit: 1) {
        owner
        media
        title
        token_id
        description
        minter
        nft_contract_icon
        nft_contract_name
      }
    }
  `;

  const [lease, setLease] = React.useState();

  React.useEffect(() => {
    async function fetchLease() {
      let lease = await leaseByContractIdAndTokenId(contractId, tokenId);
      setLease((_) => lease);
    }
    fetchLease();
  }, [contractId, tokenId]);

  console.log(lease);

  const { loading, error, data } = useQuery(GET_TOKEN, { variables: { contract_id: contractId, token_id: tokenId } });
  if (loading) return <p>Loading ...</p>
  if (error) return <p>Error</p>;

  let nft = data.mb_views_nft_tokens[0];
  if (!nft) return <p>Error: NFT info not found!</p>




  return (
    <div className="sm:flex sm:flex-row justify-between">
      <div className="sm:w-1/3 sm:px-8">
        <span className="w-full p-8 overflow-hidden">
          <img className="w-full" src={nft.media} />
        </span>
      </div>
      <div className="w-2/3 space-y-6 sm:space-y-4 sm:pl-16">
        <div className="sm:flex sm:flex-row">
          <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
            Contract Name
          </label>
          <div className="mt-1 sm:w-1/2 sm:mt-0">
            {nft.nft_contract_name}
          </div>
        </div>

        <div className="sm:flex sm:flex-row">
          <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
            Contract Id
          </label>
          <div className="mt-1 sm:w-1/2 sm:mt-0">
            {contractId}
          </div>
        </div>

        <div className="sm:flex sm:flex-row">
          <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
            Token Name
          </label>
          <div className="mt-1 sm:w-1/2 sm:mt-0">
            {nft.title}
          </div>
        </div>

        <div className="sm:flex sm:flex-row">
          <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
            Token Id
          </label>
          <div className="mt-1 sm:w-1/2 sm:mt-0">
            {tokenId}
          </div>
        </div>

        <div className="sm:flex sm:flex-row">
          <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
            Description
          </label>
          <div className="mt-1 sm:w-1/2 sm:mt-0">
            {nft.description}
          </div>
        </div>

        <div className="sm:flex sm:flex-row">
          {nft.owner == window.rentalContract.contractId ?
            <>
              <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                Currently Rented via
              </label>
              <div className="mt-1 sm:w-1/2 sm:mt-0">
                {nft.owner}
              </div>
            </>
            :
            <>
              <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                Current Owner
              </label>
              <div className="mt-1 sm:w-1/2 sm:mt-0">
                {nft.owner}
              </div>
            </>
          }
        </div>
        {lease && <>
          <div className="sm:flex sm:flex-row">
            <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
              Owner
            </label>
            <div className="mt-1 sm:w-1/2 sm:mt-0">
              {lease[1].lender_id}
            </div>
          </div>
          <div className="sm:flex sm:flex-row">
            <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
              Rented to
            </label>
            <div className="mt-1 sm:w-1/2 sm:mt-0">
              {lease[1].borrower_id}
            </div>
          </div>
        </>}
      </div>
    </div>);
}
