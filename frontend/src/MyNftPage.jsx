import React from "react";
import { useQuery, gql } from "@apollo/client";

const GET_TOKENS = gql`
    query GetTokens($account_id: String!) {
      mb_views_nft_tokens(
        where: {owner: {_eq: $account_id}, burned_timestamp: {_is_null: true}},  ) {
        owner
        media
        title
        token_id
        description
        minter
        nft_contract_icon
        nft_contract_id
        nft_contract_name
      }
    }
  `;

export default function MyNftPage() {

  const { loading, error, data } = useQuery(
    GET_TOKENS,
    { variables: { account_id: window.accountId } }
  );

  if (error) {
    console.log(error);
    return <p>Error</p>;
  }
  if (loading) return "Loading";
  const nfts_by_contract = {}
  for (let i of data.mb_views_nft_tokens) {
    let key = i.nft_contract_id;
    if (nfts_by_contract[key]) {
      nfts_by_contract[key].push(i);
    } else {
      nfts_by_contract[key] = [i];
    }
  }
  return (
    <div className="px-4 py-4 sm:px-6 lg:px-8">
      <div className="sm:flex sm:items-center mb-16">
        <div className="sm:flex-auto">
          <h1 className="text-3xl font-semibold text-gray-900">My NFTs</h1>
        </div>
      </div>
      <div className="space-y-8">
        {Object.entries(nfts_by_contract).map(([k, v], _) =>
          <div key={k}>
            <div className="text-xl mb-4">
              {v[0].nft_contract_name}
            </div>
            <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
              {
                v.map(({ token_id, title, nft_contract_id, media, }) => {
                  return <div key={nft_contract_id + "/" + token_id} className="border p-4 border-black rounded-md space-y-4">
                    <p>{title}</p>
                    <span className="h-36 w-36 overflow-hidden  bg-gray-100">
                      <img className="w-full" src={media} />
                    </span>
                    <div className="flex flex-row justify-center space-x-2">
                      <a href={"/app/nfts/" + nft_contract_id + "/" + token_id + "/lend"}
                        className="primary-btn flex-1 w-32 text-center"> Lend </a>
                      <a href={"/app/nfts/" + nft_contract_id + "/" + token_id}
                        className="btn flex-1 w-32 text-center"> Details </a>
                    </div>
                  </div>
                })
              }
            </div>
          </div>
        )}
      </div>
    </div >
  );
}
