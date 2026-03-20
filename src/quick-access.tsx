import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QuickAccess } from "./components/search/QuickAccess";
import { ToastProvider } from "./components/ui/Toast";
import "./styles/globals.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ToastProvider>
      <QuickAccess />
    </ToastProvider>
  </StrictMode>
);
