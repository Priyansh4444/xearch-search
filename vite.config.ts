import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [
    react({
      compiler: { target: "19" },
    }),
  ],
  server: { port: 5173, strictPort: true },
});
