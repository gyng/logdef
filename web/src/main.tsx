import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/global.css";

// One-shot identity stamp so you can eyeball "is this the new bundle?"
// in DevTools without diffing files. Vite injects these via define()
// at build time; see vite.config.ts.
console.log(
  `%cSupplyLine build %c${__BUILD_COMMIT__}${__BUILD_DIRTY__ ? "+dirty" : ""}%c · ${__BUILD_TIME__}`,
  "color:#d4922a;font-weight:700",
  "color:#ffd770",
  "color:#888",
);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
