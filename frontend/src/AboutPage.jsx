import React from "react";
import { topRank } from "./near-api";

export default function KarmaboardPage() {
  const [rank, setRank] = React.useState([]);

  React.useEffect(() => {
    topRank().then((r) => setRank((_) => r));
  }, []);
  console.log(rank);

  return (
    <div>
      <div className="text-2xl text-center mb-8">About Mooncake NFT</div>
      <div className="text-lg font-medium text-center mb-4">
        Twitter:{" "}
        <a
          target="_blank"
          href="https://twitter.com/mooncakenft"
          className="underline"
        >
          @MooncakeNFT
        </a>
      </div>
      {/* <div className="text-lg font-medium text-center mb-4">Discord: TBD</div> */}
    </div>
  );
}
