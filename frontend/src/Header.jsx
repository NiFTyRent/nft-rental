import React from "react";
import UserWidget from "./UserWidget";
import logoUrl from "/assets/logo.png";

export default function Header() {
  return (
    <nav>
      <div className="flex w-full items-center justify-between border-b border-indigo-500 py-6 lg:border-none">
        <div className="flex items-center">
          <a href="/">
            <img className="h-10 w-10" src={logoUrl} alt="" />
          </a>
          <div className="ml-10 hidden space-x-8 lg:block">
            <a
              href="/nft"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🤖 Mint
            </a>

            <a
              href="/karmaboard"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🍀 Karma
            </a>

            {/* <a */}
            {/*   href="#" */}
            {/*   className="text-base font-medium text-white hover:text-indigo-50" */}
            {/* > */}
            {/*   🗳 Governance */}
            {/* </a> */}

            <a
              href="/about"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🏔️ About
            </a>
          </div>
        </div>
        <div className="ml-10 space-x-4">
          <UserWidget />
        </div>
      </div>
      <div className="flex flex-wrap justify-center space-x-6 py-4 lg:hidden">
        <a
          href="/nft"
          className="text-base font-medium text-white hover:text-indigo-50"
        >
          🤖 Mint
        </a>

        <a
          href="/karmaboard"
          className="text-base font-medium text-white hover:text-indigo-50"
        >
          🍀 Karma
        </a>

        {/* <a */}
        {/*   href="#" */}
        {/*   className="text-base font-medium text-white hover:text-indigo-50" */}
        {/* > */}
        {/*   🗳 Governance */}
        {/* </a> */}

        <a
          href="/about"
          className="text-base font-medium text-white hover:text-indigo-50"
        >
          🏔️ About
        </a>
      </div>
    </nav>
  );
}
