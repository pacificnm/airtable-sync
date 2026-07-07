import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { StatusBarProvider } from "./context/StatusBarContext";
import { ToastProvider } from "./context/ToastContext";
import "./lib/fontawesome";
import "./index.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary label="root">
      <ToastProvider>
        <StatusBarProvider>
          <App />
        </StatusBarProvider>
      </ToastProvider>
    </ErrorBoundary>
  </StrictMode>,
);
