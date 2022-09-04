import React from "react";
import { Outlet } from "react-router-dom";

import Header from "./Header";

export default function App() {
  return (
    <div className="max-w-5xl px-4 sm:px-6 lg:px-8 m-auto">
      <Header />
      <Outlet />
    </div>
  );
}
