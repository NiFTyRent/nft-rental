import React from "react";
import { useParams } from "react-router-dom";
import { useQuery, gql } from "@apollo/client";
import { listingsByNftContractId } from "./near-api";
import { fromNormalisedAmount, ftSymbol } from "./FtContract"
import { contractIdToName, dateTimeString, durationString } from "./Utils";

const GET_TOKENS = gql`
    query GetTokens($nft_contract_id: String!, $nft_token_ids: [String!]!) {
      mb_views_nft_tokens(
        where: {
          nft_contract_id: {_eq: $nft_contract_id},
          token_id: {_in: $nft_token_ids},
          burned_timestamp: {_is_null: true}},
        ) {
        owner
        media
        title
        token_id
      }
    }
  `;

export default function ShopPage() {
  let { contractId } = useParams();
  let shopName = contractIdToName(contractId);
  const [listings, setListings] = React.useState([]);

  React.useEffect(() => {
    async function fetchListings() {
      listingsByNftContractId(contractId).then((listings) => {
        setListings((_) => listings)
      }
      );
    }
    fetchListings();
  }, []);

  const { loading, error, data } = useQuery(
    GET_TOKENS,
    {
      variables: {
        nft_contract_id: contractId,
        nft_token_ids: listings.map((listing) => listing.nft_token_id)
      }
    }
  );

  if (error) {
    console.log(error);
    return <p>Error</p>;
  }
  if (loading) return "Loading";

  const nft_info_by_token_id = {};
  for (let i of data.mb_views_nft_tokens) {
    nft_info_by_token_id[i.token_id] = i;
  }

  return (
    <div className="px-4 py-4 sm:px-6 lg:px-8">
      <div className="sm:flex sm:items-center mb-8">
        <div className="sm:flex-auto">
          <h1 className="text-3xl font-semibold text-gray-900">{shopName}</h1>
        </div>
      </div>
      <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
        {
          listings.map(({ nft_token_id, owner_id, price, ft_contract_id, lease_start_ts_nano, lease_end_ts_nano, }) => {
            let nft_info = nft_info_by_token_id[nft_token_id];
            return <div key={contractId + "/" + nft_token_id} className="border p-4 border-black rounded-md space-y-4">
              <p>{nft_info.title}</p>
              <span className="h-36 w-36 overflow-hidden  bg-gray-100">
                <img className="w-full" src={nft_info.media} />
              </span>
              <p className="text-center">{fromNormalisedAmount(ft_contract_id, price)} {ftSymbol(ft_contract_id)} / ~{durationString(lease_end_ts_nano - lease_start_ts_nano)}</p>
              <p className="text-center text-sm">Start from {dateTimeString(lease_start_ts_nano)} </p>
              <div className="flex flex-row justify-center space-x-2">
                <a href={"/app/listings/" + contractId + "/" + nft_token_id + "/accept"}
                  className="primary-btn flex-1 w-32 text-center"> Rent </a>
                <a href={"/app/nfts/" + contractId + "/" + nft_token_id}
                  className="btn flex-1 w-32 text-center"> Details </a>
              </div>
            </div>
          })
        }
      </div>
    </div>
  )
}