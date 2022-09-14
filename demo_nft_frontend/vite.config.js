import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  loader: { ".js": "jsx" },
  // To read contract name
  envPrefix: "CONTRACT_",
});
