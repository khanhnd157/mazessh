import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App";
import { ConsentPopup } from "./components/consent/ConsentPopup";
import "./globals.css";

const windowLabel = getCurrentWindow().label;

const root = document.getElementById("root") as HTMLElement;

if (windowLabel === "consent") {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <ConsentPopup />
    </React.StrictMode>,
  );
} else {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
