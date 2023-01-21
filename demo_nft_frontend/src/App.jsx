import { Outlet } from "react-router-dom";
import UserWidget from "./UserWidget";

export default function App() {
  return (
    <main>
      <UserWidget />
      <Outlet />
    </main>
  );
}
