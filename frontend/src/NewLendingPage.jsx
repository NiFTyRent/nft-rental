import React from "react";
import AutoInput from "./AutoInput";
import { useQuery, gql } from "@apollo/client";
import { initContract, nftTokensForOwner, newLease } from "./NftContract";

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

export default function NewLendingPage() {
  const { loading, error, data } = useQuery(GET_CONTRACTS);
  const [selectedContract, setSelectedContract] = React.useState("");
  const [selectedToken, setSelectedToken] = React.useState("");
  const [tokens, setTokens] = React.useState([]);
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

  React.useEffect(() => {
    async function fetchTokens() {
      let contract = await initContract(selectedContract.id);

      nftTokensForOwner(contract, window.accountId).then((tokens) =>
        setTokens((_) => tokens)
      );
    }
    if (selectedContract == "") return;
    fetchTokens();
  }, [selectedContract]);

  if (loading) return <p>Loading...</p>;
  if (error) return <p>Error :(</p>;

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
                <div>
                  <h3 className="text-lg font-medium leading-6 text-gray-900">
                    NFT Info
                  </h3>
                </div>

                <div>
                  <div className="flex flex-row space-x-8 justify-between">
                    <div className="flex-auto space-y-6 sm:space-y-5">
                      <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                        <label
                          htmlFor="contract_addr"
                          className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2"
                        >
                          Contract
                        </label>
                        <div className="mt-1 sm:col-span-2 sm:mt-0">
                          <AutoInput
                            className="block w-full min-w-0 flex-1 rounded-none rounded-r-md border-gray-300 focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                            selected={selectedContract}
                            setSelected={setSelectedContract}
                            options={data.nft_contracts.map(({ id, name }) => {
                              return { id, name };
                            })}
                          />
                          <p className="mt-2 text-sm text-gray-500">
                            Choose the contract of your NFT.
                          </p>
                        </div>
                      </div>

                      <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                        <label
                          htmlFor="token_id"
                          className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2"
                        >
                          Token
                        </label>
                        <div className="mt-1 sm:col-span-2 sm:mt-0">
                          <AutoInput
                            className="block w-full min-w-0 flex-1 rounded-none rounded-r-md border-gray-300 focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                            selected={selectedToken}
                            setSelected={setSelectedToken}
                            options={tokens.map(({ token_id, metadata }) => {
                              return {
                                id: token_id,
                                name: metadata.title || token_id,
                                media: metadata.media,
                              };
                            })}
                          />
                          <p className="mt-2 text-sm text-gray-500">
                            Choose the token you want to lend
                          </p>
                        </div>
                      </div>
                    </div>

                    <div className="flex-none">
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

                  <div>
                    <h3 className="text-lg font-medium leading-6 text-gray-900">
                      Lease Info
                    </h3>
                  </div>

                  <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                    <label className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2">
                      Borrower
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      <input
                        type="text"
                        className="block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                        value={borrower}
                        onChange={(e) => setBorrower(e.target.value)}
                      />
                      <p className="mt-2 text-sm text-gray-500">
                        The account you want to lend the NFT to
                      </p>
                    </div>
                  </div>

                  <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                    <label className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2">
                      Rent Duration
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      <div className="flex flex-row space-x-2">
                        <div>
                          <div className="text-sm">Days</div>
                          <input
                            type="number"
                            className="block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                            value={durationDay}
                            onChange={(e) => setDurationDay(e.target.value)}
                          />
                        </div>
                        <div>
                          <div className="text-sm">Hours</div>
                          <input
                            type="number"
                            className="block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
                            value={durationHour}
                            onChange={(e) => setDurationHour(e.target.value)}
                          />
                        </div>
                        <div>
                          <div className="text-sm">Minutse</div>
                          <input
                            type="number"
                            className="block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
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

                  <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                    <label className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2">
                      Rent
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      <input
                        type="number"
                        className="block w-full max-w-lg rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm"
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
