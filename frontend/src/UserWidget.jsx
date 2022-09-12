import React from "react";
import { Fragment, useState } from "react";
import { Menu, Transition } from "@headlessui/react";
import { classNames } from "./Utils";
import { signInWithNearWallet, signOutNearWallet } from "./near-api";

export default function UserWidget() {
  let [showSignOut, setShowSignOut] = React.useState(false);

  return window.walletConnection.isSignedIn() ? (
    <>
      <Menu as="div" className="relative ml-3">
        <div>
          <Menu.Button className="flex max-w-xs p-2 items-center rounded-md bg-white text-sm focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-2">
            {window.accountId}
          </Menu.Button>
        </div>
        <Transition
          as={Fragment}
          enter="transition ease-out duration-100"
          enterFrom="transform opacity-0 scale-95"
          enterTo="transform opacity-100 scale-100"
          leave="transition ease-in duration-75"
          leaveFrom="transform opacity-100 scale-100"
          leaveTo="transform opacity-0 scale-95"
        >
          <Menu.Items className="absolute right-0 z-10 mt-2 w-48 origin-top-right rounded-md bg-white py-1 shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none">
            <Menu.Item>
              {({ active }) => (
                <div
                  className={classNames(
                    active ? "bg-gray-200" : "",
                    "block px-4 py-2 text-sm text-gray-700"
                  )}
                  onClick={signOutNearWallet}
                >
                  Sign out
                </div>
              )}
            </Menu.Item>
          </Menu.Items>
        </Transition>
      </Menu>
    </>
  ) : (
    <button
      onClick={signInWithNearWallet}
      className="inline-block rounded-md border border-transparent bg-indigo-500 py-2 px-4 text-base font-medium text-white hover:bg-opacity-75"
    >
      Sign in
    </button>
  );
}
