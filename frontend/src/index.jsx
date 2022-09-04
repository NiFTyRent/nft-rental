import "regenerator-runtime/runtime";
import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import App from "./App";
import Home from "./Home";
import NftPage from "./NftPage";
import KarmaboardPage from "./KarmaboardPage";
import AboutPage from "./AboutPage";
import MyNftsPage from "./MyNftsPage";
import { initContract } from "./near-api";

const reactRoot = createRoot(document.querySelector("#root"));

window.nearInitPromise = initContract()
  .then(() => {
    reactRoot.render(
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<App />}>
            <Route index element={<Home />} />
            <Route path="nft" element={<NftPage />} />
            <Route path="karmaboard" element={<KarmaboardPage />} />
            <Route path="about" element={<AboutPage />} />
            <Route path="mynfts" element={<MyNftsPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    );
  })
  .catch((e) => {
    reactRoot.render(
      <div style={{ color: "red" }}>
        Error: <code>{e.message}</code>
      </div>
    );
    console.error(e);
  });
