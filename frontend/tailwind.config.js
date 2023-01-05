/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.{html,js,jsx,ts,tsx}", "./index.html"],
  theme: {
    fontFamily: {
      "sans": ['"Share Tech Mono"', 'monospace'],
      "body": ['"Share Tech Mono"', 'monospace'],
    },
    extend: {
      colors: {
        // accent: "#D26666",
      }
    },
  },
  plugins: [require("@tailwindcss/forms")],
};
