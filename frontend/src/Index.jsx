import "regenerator-runtime/runtime";
import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import Home from "./Home";
import App from "./App";
import AppHome from "./AppHome";
import NewLendingPage from "./NewLendingPage";
import AcceptBorrowingPage from "./AcceptBorrowingPage";
import BorrowingsPage from "./BorrowingsPage";
import LendingsPage from "./LendingsPage";
import { initContract, getAllowedFTs } from "./near-api";
import {
  ApolloClient,
  InMemoryCache,
  ApolloProvider,
} from "@apollo/client";

const reactRoot = createRoot(document.querySelector("#root"));

async function render() {
  try {
    await initContract();
    window.CURRENCY_OPTIONS = await getAllowedFTs();
    reactRoot.render(
      <ApolloProvider client={mintbaseClient}>
        <BrowserRouter>
          <Routes>
            <Route exact path="/" element={<Home />} />
            <Route path="/app" element={<App />}>
              <Route index element={<AppHome />} />
              <Route path="lendings" element={<LendingsPage />} />
              <Route path="lendings/new" element={<NewLendingPage />} />
              <Route path="borrowings" element={<BorrowingsPage />} />
              <Route
                path="borrowings/:leaseId/accept"
                element={<AcceptBorrowingPage />}
              />
            </Route>
          </Routes>
        </BrowserRouter>
      </ApolloProvider>);
  } catch (e) {
    reactRoot.render(
      <div style={{ color: "red" }}>
        Error: <code>{e.message}</code>
      </div>
    );
    console.error(e);
  }
}

const mintbaseClient = new ApolloClient({
  uri: "https://interop-testnet.hasura.app/v1/graphql",
  cache: new InMemoryCache(),
});

render();
