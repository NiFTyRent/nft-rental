import React from "react";
import { myBorrowings, acceptLease } from "./near-api";
import { initContract, getToken } from "./NftContract";
import { useParams } from "react-router-dom";

export default function AcceptBorrowingPage() {
  let { leaseId } = useParams();
  const [borrowing, setBorrowing] = React.useState(null);
  const [media, setMedia] = React.useState("");

  React.useEffect(() => {
    async function fetchBorrowings() {
      let borrowings = await myBorrowings(window.accountId);
      let borrowing = borrowings.find(([k, _]) => {
        return k == leaseId;
      });

      setBorrowing((_) => borrowing);
      if (borrowing) {
        let contract = await initContract(borrowing[1].contract_addr);

        let token = await getToken(contract, borrowing[1].token_id);
        if (token) {
          setMedia(token?.metadata?.media);
        }
      }
    }
    fetchBorrowings();
  }, []);

  let onSubmit = () => {
    console.log(acceptLease(leaseId, borrowing[1].price));
  };

  return borrowing ? (
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
                          {borrowing[1].contract_addr}
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
                          {borrowing[1].token_id}
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
                            <img src={media} />
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
                      {borrowing[1].borrower}
                    </div>
                  </div>

                  <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                    <label className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2">
                      Expiration Time
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      <div className="flex flex-row space-x-2">
                        <div>
                          {new Date(
                            borrowing[1].expiration * 1000
                          ).toLocaleString()}
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="sm:grid sm:grid-cols-3 sm:items-start sm:gap-4 sm:border-t sm:border-gray-200 sm:pt-5">
                    <label className="block text-sm font-medium text-gray-700 sm:mt-px sm:pt-2">
                      Rent
                    </label>
                    <div className="mt-1 sm:col-span-2 sm:mt-0">
                      {window.nearApi.utils.format.formatNearAmount(
                        BigInt(borrowing[1].price).toString()
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div className="pt-5">
              {borrowing[1].state == "Pending" ? (
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
                    Accept
                  </button>
                </div>
              ) : (
                <div>The lease has been approved</div>
              )}
            </div>
          </div>
        </div>
      </div>
    </>
  ) : (
    "Loading"
  );
}
