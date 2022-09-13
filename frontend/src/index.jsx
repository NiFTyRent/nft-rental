import "regenerator-runtime/runtime";
import React from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import App from "./App";
import Home from "./Home";
import NewLendingPage from "./NewLendingPage";
import AcceptBorrowingPage from "./AcceptBorrowingPage";
import BorrowingsPage from "./BorrowingsPage";
import { initContract } from "./near-api";
import {
  ApolloClient,
  InMemoryCache,
  ApolloProvider,
  gql,
} from "@apollo/client";

const reactRoot = createRoot(document.querySelector("#root"));

window.nearInitPromise = initContract()
  .then(() => {
    reactRoot.render(
      <ApolloProvider client={mintbaseClient}>
        <BrowserRouter>
          <Routes>
            <Route path="/" element={<App />}>
              <Route index element={<Home />} />
              <Route path="lendings/new" element={<NewLendingPage />} />
              <Route path="borrowings" element={<BorrowingsPage />} />
              <Route
                path="borrowings/:leaseId/accept"
                element={<AcceptBorrowingPage />}
              />
            </Route>
          </Routes>
        </BrowserRouter>
      </ApolloProvider>
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

const mintbaseClient = new ApolloClient({
  uri: "https://interop-testnet.hasura.app/v1/graphql",
  cache: new InMemoryCache(),
});
