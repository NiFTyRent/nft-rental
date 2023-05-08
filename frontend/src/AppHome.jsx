import React from "react";

export default function AppHome() {
  return (
    <>
      <div className="py-6 bg-zinc-200">
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <h1 className="text-2xl mb-8 font-semibold">Home</h1>
        </div>
        <div className="mx-auto max-w-7xl px-4 sm:px-6 md:px-8">
          <div className="flex flex-col space-y-4 max-w-md mx-auto">
            <a href="/app/shops" className="primary-btn" > All Shops </a>
            <a href="/app/nfts" className="btn" > My NFTs </a>
          </div>
        </div>
      </div>
    </>
  );
}
