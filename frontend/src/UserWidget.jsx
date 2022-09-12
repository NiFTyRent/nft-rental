import { signInWithNearWallet, signOutNearWallet } from "./near-api";
import React from "react";

export default function UserWidget() {
  let [showSignOut, setShowSignOut] = React.useState(false);
  return window.walletConnection.isSignedIn() ? (
    showSignOut ? (
      <div
        className="inline-block rounded-md border border-indigo-500 py-2 px-4 text-base font-medium hover:bg-indigo-500 cursor-pointer"
        onClick={signOutNearWallet}
      >
        Sign out
      </div>
    ) : (
      <div
        className="inline-block rounded-md border border-indigo-500 py-2 px-4 text-base font-medium hover:bg-indigo-500 cursor-pointer"
        onClick={(_) => {
          setShowSignOut((_) => true);
          setTimeout((_) => setShowSignOut((_) => false), 3000);
        }}
      >
        {window.accountId}
      </div>
    )
  ) : (
    <button
      onClick={signInWithNearWallet}
      className="inline-block rounded-md border border-transparent bg-indigo-500 py-2 px-4 text-base font-medium text-white hover:bg-opacity-75"
    >
      Sign in
    </button>
  );
}
