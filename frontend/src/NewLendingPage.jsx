import React from "react";
import AutoInput from "./AutoInput";
import { useQuery, gql } from "@apollo/client";
import { initContract, newLease } from "./NftContract";



function ContractAutoInput({ className, query, selected, setSelected }) {
  const GET_CONTRACTS = gql`
    query GetContracts {
      nft_contracts {
        id
        name
        is_mintbase
        base_uri
      }
    }
  `;

  const { loading, error, data } = useQuery(GET_CONTRACTS, { name: query });
  if (loading) return <AutoInput className={className} loading={true} />;
  if (error) return <p>Error</p>;

  return <AutoInput
    className={className}
    selected={selected}
    setSelected={setSelected}
    options={data.nft_contracts.map(({ id, name }) =>
      ({ id, name: `${name} (${id})` })
    )}
  />;
}

function TokenAutoInput({ className, contractId, selected, setSelected }) {
  const GET_TOKENS = gql`
    query GetTokens($contract_id: String!) {
      nft_metadata(where: {nft_contract: {id: {_eq: $contract_id}}}) {
        id
        media
        title
      }
      nft_tokens(where: {nft_contract: {id: {_eq: $contract_id}}}) {
        metadata_id
        owner
        token_id
      }
    }
  `;

  if (!contractId) return <AutoInput className={className} disabled={true} />

  const { loading, error, data } = useQuery(GET_TOKENS, { variables: { contract_id: contractId } });
  if (loading) return <AutoInput className={className} loading={true} />;
  if (error) return <p>Error</p>;

  let metadata_by_id = new Map();
  for (let m of data.nft_metadata) {
    metadata_by_id.set(m.id, m)
  }

  // TODO(libo): hide the token not owned by the user.
  return <AutoInput
    className={className}
    selected={selected}
    setSelected={setSelected}
    options={data.nft_tokens.map(({ token_id, metadata_id }) => {
      let metadata = metadata_by_id.get(metadata_id);
      return {
        id: token_id,
        name: metadata?.title || id,
        media: metadata?.media,
      }
    }
    )}
  />;
}

export default function NewLendingPage() {
  const [selectedContract, setSelectedContract] = React.useState("");
  const [selectedToken, setSelectedToken] = React.useState("");
  const [borrower, setBorrower] = React.useState("");
  const [durationMinute, setDurationMinute] = React.useState(0);
  const [durationHour, setDurationHour] = React.useState(0);
  const [durationDay, setDurationDay] = React.useState(0);
  const [rent, setRent] = React.useState(0);

  let onSubmit = async () => {
    let contract = await initContract(selectedContract.id);
    let expiration =
      Math.trunc(Date.now() / 1000) +
      durationDay * 24 * 3600 +
      durationHour * 3600 +
      durationMinute * 60;

    newLease(contract, selectedToken.id, borrower, expiration, rent);
  };

  return (
    <>
      <div className="py-6">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <h1 className="text-2xl mb-8 font-semibold text-gray-900">
            New Lease
          </h1>
        </div>

        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <div className="space-y-8 divide-y divide-gray-200">
            <div className="flex flex-col space-y-8 divide-y divide-gray-200 sm:space-y-5">

              <div className="space-y-6 sm:space-y-5">
                <h3 className="text-lg font-medium leading-6 text-gray-900">
                  NFT Info
                </h3>

                <div className="sm:flex sm:flex-row justify-between">
                  <div className="w-2/3 space-y-6 sm:space-y-5">

                    <div className="sm:flex sm:flex-row">
                      <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                        Contract
                      </label>
                      <div className="mt-1 sm:w-1/2 sm:mt-0">
                        <ContractAutoInput
                          className="input max-w-lg"
                          selected={selectedContract}
                          setSelected={setSelectedContract}
                        />
                        <p className="mt-2 text-sm text-gray-500">
                          Choose the contract of your NFT.
                        </p>
                      </div>
                    </div>

                    <div className="sm:flex sm:flex-row">
                      <label className="block sm:w-1/2 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                        Token
                      </label>
                      <div className="mt-1 sm:w-1/2 sm:mt-0">
                        <TokenAutoInput
                          className="input max-w-lg"
                          contractId={selectedContract.id}
                          selected={selectedToken}
                          setSelected={setSelectedToken}
                        />
                        <p className="mt-2 text-sm text-gray-500">
                          Choose the token you want to lend
                        </p>
                      </div>
                    </div>
                  </div>

                  <div className="sm:w-1/3 sm:px-8">
                    <label className="block text-sm font-medium text-gray-700 pb-4">
                      NFT Image
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      <div className="flex items-center">
                        <span className="h-36 w-36 overflow-hidden  bg-gray-100">
                          <img src={selectedToken?.media} />
                        </span>
                      </div>
                    </div>
                  </div>
                </div>

                <div className="space-y-6 sm:space-y-5">
                  <h3 className="text-lg font-medium leading-6 text-gray-900">
                    Lease Info
                  </h3>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Borrower
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      <input
                        type="text"
                        className="input max-w-lg"
                        value={borrower}
                        onChange={(e) => setBorrower(e.target.value)}
                      />
                      <p className="mt-2 text-sm text-gray-500">
                        The account you want to lend the NFT to
                      </p>
                    </div>
                  </div>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Rent Duration
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      <div className="flex flex-row space-x-2 max-w-lg">
                        <div>
                          <div className="text-sm">Days</div>
                          <input
                            type="number"
                            className={"input" + (durationDay >= 0 ? "" : " input-error")}
                            value={durationDay}
                            onChange={(e) => setDurationDay(e.target.value)}
                          />
                        </div>
                        <div>
                          <div className="text-sm">Hours</div>
                          <input
                            type="number"
                            className={"input" + (durationHour >= 0 ? "" : " input-error")}
                            value={durationHour}
                            onChange={(e) => setDurationHour(e.target.value)}
                          />
                        </div>
                        <div>
                          <div className="text-sm">Minutes</div>
                          <input
                            type="number"
                            className={"input" + (durationMinute >= 0 ? "" : " input-error")}
                            value={durationMinute}
                            onChange={(e) => setDurationMinute(e.target.value)}
                          />
                        </div>
                      </div>
                      <p className="mt-2 text-sm text-gray-500">
                        How long you want to rent your NFT
                      </p>
                    </div>
                  </div>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Rent
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      <input
                        type="number"
                        className={"input max-w-lg" + (rent >= 0 ? "" : " input-error")}
                        value={rent}
                        onChange={(e) => setRent(e.target.value)}
                      />
                      <p className="mt-2 text-sm text-gray-500">
                        How much rent the borrower should pay you (in NEAR)
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="pt-5">
              <div className="flex justify-end space-x-4">
                <a className="btn" href="/app" >
                  Cancel
                </a>
                <button className="primary-btn" onClick={(_) => onSubmit()} >
                  Submit
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
