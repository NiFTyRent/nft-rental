import React from "react";

import { CurrencySelector } from "./CurrencySelector";
import { useParams } from "react-router-dom";
import { initContract, newListing, getPayout } from "./NftContract";
import { fromNormalisedAmount, initFtContract, toNormalisedAmount } from "./FtContract";
import { NftInfo } from "./NftInfo";
import { MS_TO_NS_SCALE } from "./Utils";


export default function NewLendingPage() {
  const { contractId, tokenId } = useParams();
  const [startTimeStr, setStartTimeStr] = React.useState("");
  const [endTimeStr, setEndTimeStr] = React.useState("");
  const [rentCurrency, setRentCurrency] = React.useState(window.CURRENCY_OPTIONS[0]);
  const [rent, setRent] = React.useState(0);
  const [royalty, setRoyalty] = React.useState(0);
  // TODO(libo): Set marketplace fee
  const [marketFee, setMarketFee] = React.useState(0);
  const [validationErrors, setValidationErrors] = React.useState({});

  let validate = () => {
    const errors = {}
    if (!startTimeStr) {
      errors.startTimeStr = "Please set lease start time";
    }
    if (!endTimeStr) {
      errors.endTimeStr = "Please set lease end time";
    }
    const startTime = new Date(startTimeStr);
    const endTime = new Date(endTimeStr);
    if (startTime >= endTime) {
      errors.endTimeStr = "Lease end time must be later than the start time";
    }
    if (!rent || rent <= 0) {
      errors.rent = "Please set a positive number for the rent price";
    }
    return errors;
  }


  let calcuateRoyaltySplit = async () => {
    const contract = await initContract(contractId);
    const priceNormalised = toNormalisedAmount(rentCurrency.address, rent);


    const payouts = await getPayout(contract, tokenId, priceNormalised);
    let royalty = 0;
    for (let accountId of Object.keys(payouts)) {
      if (accountId != window.accountId) {
        royalty += payouts[accountId];
      }
    }

    return fromNormalisedAmount(rentCurrency.address, royalty);
  }

  React.useEffect(() => {
    (async () => setRoyalty(await calcuateRoyaltySplit()))();
  }, [rent])

  let onSubmit = async () => {
    const errors = validate();
    setValidationErrors(errors)
    if (Object.keys(errors).length > 0) {
      return;
    }
    const contract = await initContract(contractId);
    const startTsNano = new Date(startTimeStr).valueOf() * MS_TO_NS_SCALE;
    const endTsNano = new Date(endTimeStr).valueOf() * MS_TO_NS_SCALE;

    newListing(contract, tokenId, startTsNano, endTsNano, rentCurrency.address, rent);
  };

  let errorMessage = (message) => {
    return message ? <p className="mt-2 text-sm text-red-500"> {message} </p> : null
  }

  return (
    <>
      <div className="py-6">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <h1 className="text-2xl mb-8 font-semibold text-gray-900">
            Lent My NFT
          </h1>
        </div>

        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <div className="space-y-8 divide-y divide-gray-200">
            <div className="flex flex-col space-y-8 divide-y divide-gray-200">

              <NftInfo contractId={contractId} tokenId={tokenId} />

              <div className="space-y-6 sm:space-y-4">
                <div className="sm:flex sm:flex-row">
                  <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                    Start Time
                  </label>
                  <div className="mt-1 sm:w-2/3 sm:mt-0 max-w-lg">
                    {errorMessage(validationErrors["startTimeStr"])}
                    <input className="input" type="datetime-local" value={startTimeStr} onChange={(e) => setStartTimeStr(e.target.value)} />
                    <p className="mt-2 text-sm text-gray-500">
                      When do you want to start renting your NFT
                    </p>
                  </div>
                </div>
                <div className="sm:flex sm:flex-row">
                  <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                    End Time
                  </label>
                  <div className="mt-1 sm:w-2/3 sm:mt-0 max-w-lg">
                    {errorMessage(validationErrors["endTimeStr"])}
                    <input className="input" type="datetime-local" value={endTimeStr} onChange={(e) => setEndTimeStr(e.target.value)} />
                    <p className="mt-2 text-sm text-gray-500">
                      When do you want your NFT to be returned
                    </p>
                  </div>
                </div>
                <div className="sm:flex sm:flex-row">
                  <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                    Rent
                  </label>
                  <div className="mt-1 sm:w-2/3 sm:mt-0 max-w-lg">
                    {errorMessage(validationErrors["rent"])}
                    <div className="flex flex-row space-x-2">
                      <div className="w-1/3">
                        <CurrencySelector
                          selected={rentCurrency}
                          setSelected={setRentCurrency} />
                      </div>
                      <input
                        type="number"
                        className={"w-2/3 input" + (rent >= 0 ? "" : " input-error")}
                        value={rent}
                        onChange={(e) => setRent(e.target.value)}
                      />
                    </div>
                    <p className="mt-2 text-sm text-gray-500">
                      How much rent the borrower should pay you
                    </p>
                  </div>
                </div>

                <div className="sm:flex sm:flex-row">
                  <label htmlFor="contract_addr" className="block sm:w-1/3 text-sm font-medium text-gray-700 sm:mt-px sm:pt-2" >
                    Fees
                  </label>
                  <div className="mt-1 sm:w-2/3 sm:mt-0 max-w-lg">
                    <div className="flex flex-col space-y-2">
                      <div>Royalty .......................... {royalty} {rentCurrency.symbol}</div>
                      <div>Market ........................... {marketFee} {rentCurrency.symbol}</div>
                      <div>Total ............................ {marketFee + royalty} {rentCurrency.symbol}</div>
                    </div>
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
    </>
  );
}
