import React from "react";
import { myBorrowings, acceptLease } from "./near-api";
import { initContract, getToken } from "./NftContract";
import {
  initContract as initRentalContract,
  getBorrower,
} from "./RentalContract";
import { useParams } from "react-router-dom";

export default function PetPage() {
  let { contractId, petId } = useParams();

  const [isOwner, setIsOwner] = React.useState(false);
  const [message, setMessage] = React.useState("");
  const [eyes, setEyes] = React.useState("oo");
  React.useEffect(() => {
    async function fetchTokens() {
      let contract = await initContract(contractId);
      let token = await getToken(contract, petId);
      setMessage(`I'm ${token.metadata.title}`);
      if (token) {
        if (token.owner_id == window.accountId) {
          setIsOwner(true);
          return;
        }
        let rentalContract = await initRentalContract(token.owner_id);
        let borrower = await getBorrower(rentalContract, contractId, petId);
        if (borrower && borrower == window.accountId) {
          setIsOwner(true);
          return;
        }
      }
    }
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
