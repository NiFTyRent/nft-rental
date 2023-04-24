import React from "react";
import { nearConfig, myBorrowings } from "./near-api";
import { initFtContract } from "./FtContract";
import { NftInfo } from "./NftInfo";

export default function AcceptBorrowingPage() {
  let { leaseId } = useParams();
  const [borrowing, setBorrowing] = React.useState(null);

  React.useEffect(() => {
    async function fetchBorrowings() {
      let borrowings = await myBorrowings(window.accountId);
      let borrowing = borrowings.find(([k, _]) => {
        return k == leaseId;
      });

      const ftContract = await initFtContract(borrowing[1].ft_contract_addr);
      const ftMetadata = await ftContract.ft_metadata();
      const ftDecimals = ftMetadata.decimals;
      borrowing[1].uiPrice = Number(BigInt(borrowing[1].price) / (BigInt(10) ** BigInt(ftDecimals - 3))) / 1000;
      borrowing[1].symbol = ftMetadata.symbol;

      setBorrowing((_) => borrowing);
    }
    fetchBorrowings();
  }, [leaseId]);

  let onSubmit = async () => {
    if (!borrowing) return;
    const ftContract = await initFtContract(borrowing[1].ft_contract_addr);
    const amount = BigInt(borrowing[1].price).toString();
    return await ftContract.ft_transfer_call({
      args: {
        receiver_id: nearConfig.contractName,
        amount: amount,
        msg: JSON.stringify({ lease_id: borrowing[0] })
      },
      gas: "300000000000000",
      amount: "1",
    })
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
            <div className="flex flex-col space-y-8 divide-y divide-gray-200">
              <div className="space-y-6">
                <h3 className="text-lg font-medium leading-6 text-gray-900">
                  NFT Info
                </h3>

                <NftInfo contractId={borrowing[1].contract_addr} tokenId={borrowing[1].token_id} />

                <h3 className="text-lg font-medium leading-6 text-gray-900">
                  Lease Info
                </h3>

                <div className="space-y-6 sm:space-y-4">
                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Borrower
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {borrowing[1].borrower_id}
                    </div>
                  </div>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Expiration Time
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {new Date(
                        borrowing[1].expiration * 1000
                      ).toLocaleString()}
                    </div>
                  </div>

                  <div className="sm:flex sm:flex-row">
                    <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                      Rent
                    </label>
                    <div className="mt-1 sm:w-2/3 sm:mt-0">
                      {borrowing[1].uiPrice} {borrowing[1].symbol}
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
      </div >
    </>
  ) : (
    "Loading"
  );
}
