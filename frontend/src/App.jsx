import "regenerator-runtime/runtime";
import React from "react";

import { getGreetingFromContract, setGreetingOnContract } from "./near-api";
import { EducationalText, SignInPrompt, SignOutButton } from "./ui-components";

export default function App() {
  const [valueFromBlockchain, setValueFromBlockchain] = React.useState();

  const [uiPleaseWait, setUiPleaseWait] = React.useState(true);

  // Get blockchian state once on component load
  // React.useEffect(() => {
  //   getGreetingFromContract()
  //     .then(setValueFromBlockchain)
  //     .catch(alert)
  //     .finally(() => {
  //       setUiPleaseWait(false);
  //     });
  // }, []);

  /// If user not signed-in with wallet - show prompt

  function changeGreeting(e) {
    e.preventDefault();
    setUiPleaseWait(true);
    const { greetingInput } = e.target.elements;
    setGreetingOnContract(greetingInput.value)
      .then(getGreetingFromContract)
      .then(setValueFromBlockchain)
      .catch(alert)
      .finally(() => {
        setUiPleaseWait(false);
      });
  }

  return (
    <div>
      <div className="max-w-5xl p-8 m-auto">
        <nav
          className="mx-auto max-w-7xl px-4 sm:px-6 lg:px-8"
          aria-label="Top"
        >
          <div className="flex w-full items-center justify-between border-b border-indigo-500 py-6 lg:border-none">
            <div className="flex items-center">
              <a href="#">
                <img className="h-10 w-10" src="/assets/logo.png" alt="" />
              </a>
              <div className="ml-10 hidden space-x-8 lg:block">
                <a
                  href="#"
                  className="text-base font-medium text-white hover:text-indigo-50"
                >
                  🤖 Explore
                </a>

                <a
                  href="#"
                  className="text-base font-medium text-white hover:text-indigo-50"
                >
                  🗳 Governance
                </a>

                <a
                  href="#"
                  className="text-base font-medium text-white hover:text-indigo-50"
                >
                  🏔️ About
                </a>
              </div>
            </div>
            <div className="ml-10 space-x-4">
              <a
                href="#"
                className="inline-block rounded-md border border-transparent bg-indigo-500 py-2 px-4 text-base font-medium text-white hover:bg-opacity-75"
              >
                Sign in!
              </a>
            </div>
          </div>
          <div className="flex flex-wrap justify-center space-x-6 py-4 lg:hidden">
            <a
              href="#"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🤖 Explore
            </a>

            <a
              href="#"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🗳 Governance
            </a>

            <a
              href="#"
              className="text-base font-medium text-white hover:text-indigo-50"
            >
              🏔️ About
            </a>
          </div>
        </nav>
        <div className="flex flex-row mb-8 space-x-4 justify-between items-start">
          <div className="flex-1 space-y-2">
            <div className="text-3xl font-serif">Mooncake</div>
            <div className="text-base italic font-serif">noun</div>
            <div className="text-base">
              A Chinese bakery product traditionally eaten during the{" "}
              <a
                href="https://www.google.com/search?q=mid-autumn+festival"
                target="_blank"
                className="underline text-indigo-400"
              >
                Mid-Autumn Festival
              </a>
              . Often been gifted to family and friends to give best wishes.
            </div>
          </div>
          <div className="flex-1 space-y-2">
            <div className="text-3xl font-serif">Mooncake NFT</div>
            <div className="text-base italic font-serif">noun</div>
            <div className="text-base">
              a humble virtual (and on chain) Mooncake. Dairy free. Zero
              calories. Keep you in a good mood when added to your wallet. Also
              a great gift for your (crypto) friends.
            </div>
          </div>
        </div>
        {/* {window.walletConnection.isSignedIn() ? ( */}
        {/*   <> */}
        {/*     <SignOutButton accountId={window.accountId} /> */}
        {/*     <main className={uiPleaseWait ? "please-wait" : ""}> */}
        {/*       <h1> */}
        {/*         The contract says:{" "} */}
        {/*         <span className="greeting">{valueFromBlockchain}</span> */}
        {/*       </h1> */}
        {/*       <form onSubmit={changeGreeting} className="change"> */}
        {/*         <label>Change greeting:</label> */}
        {/*         <div> */}
        {/*           <input */}
        {/*             autoComplete="off" */}
        {/*             defaultValue={valueFromBlockchain} */}
        {/*             id="greetingInput" */}
        {/*           /> */}
        {/*           <button> */}
        {/*             <span>Save</span> */}
        {/*             <div className="loader"></div> */}
        {/*           </button> */}
        {/*         </div> */}
        {/*       </form> */}
        {/*       <EducationalText /> */}
        {/*     </main> */}
        {/*   </> */}
        {/* ) : ( */}
        {/*   // Sign-in flow will reload the page later */}
        {/*   <SignInPrompt greeting={valueFromBlockchain} /> */}
        {/* )} */}
      </div>
    </div>
  );
}
