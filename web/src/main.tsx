import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "./App";
import "./styles/global.css";

// Build identity, so "is this the new bundle?" is answerable from
// DevTools without diffing files. Vite injects these; see vite.config.ts.
console.log(
  `%cUnderstory %c${__BUILD_COMMIT__}${__BUILD_DIRTY__ ? "+dirty" : ""}%c · ${__BUILD_TIME__}`,
  "color:#d9ad5e;font-weight:700",
  "color:#8fc46a",
  "color:#6a8073",
);

const root = document.getElementById("root");
if (!root) throw new Error("index.html is missing #root");

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
