import React from "react";
import { NiftyRent } from "@niftyrent/sdk"
import { useParams } from "react-router-dom";


export default function PetPage() {
  let { contractId, petId } = useParams();

  const [isOwner, setIsOwner] = React.useState(false);
  const [message, setMessage] = React.useState("");
  const [eyes, setEyes] = React.useState("oo");
  React.useEffect(() => {
    async function fetchTokens() {
      let niftyrent = new NiftyRent({
        defaultContractAddr: contractId,
        allowedRentalProxies: ["nft-rental.testnet"],
      });
      await niftyrent.init();

      if (await niftyrent.is_current_user(petId, window.accountId)) {
        setIsOwner(true);
      }
    }

    setMessage(`Hello`);
    fetchTokens();
  }, []);

  return (
    <div>
      <pre>
        {String.raw`
   ________________________________________
  < ${message}${" ".repeat(38 - message.length)} >
   ----------------------------------------
          \   ^__^
           \  (${eyes})\_______
              (__)\       )\/\
                  ||----w |
                  ||     ||
              `}
      </pre>
      {isOwner ? (
        <button
          onClick={(_) => {
            setMessage("MOO~~~ MOO~~~ MOO~~~~~~~");
            setEyes("><");
          }}
        >
          <pre>
            {`
              +---------+
              |   Pat   |
              +---------+
            `}
          </pre>
        </button>
      ) : (
        <div>Only the owner can pat me</div>
      )}
    </div>
  );
}
