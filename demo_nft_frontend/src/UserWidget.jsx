import React from "react";
import { Fragment, useState } from "react";
import { signInWithNearWallet, signOutNearWallet } from "./near-api";

export default function UserWidget() {
  let [showSignOut, setShowSignOut] = React.useState(false);

  return window.walletConnection.isSignedIn() ? (
    <button onClick={signInWithNearWallet}>
      <pre>
        {`
              +---------------+
              | ${window.accountId} |
              +---------------+
            `}
      </pre>
    </button>
  ) : (
    <button onClick={signInWithNearWallet}>
      <pre>
        {`
              +---------+
              | Sign in |
              +---------+
            `}
      </pre>
    </button>
  );
}
