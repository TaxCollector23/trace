import React from "react";
import ReactDOM from "react-dom/client";
import { createHashRouter, RouterProvider } from "react-router-dom";
import App from "./App";
import Dashboard from "./pages/Dashboard";
import RunTimeline from "./pages/RunTimeline";
import PatchReview from "./pages/PatchReview";
import CostCenter from "./pages/CostCenter";
import RiskCenter from "./pages/RiskCenter";
import RollbackCenter from "./pages/RollbackCenter";
import GitHub from "./pages/GitHub";
import NotFound from "./pages/NotFound";
import "./styles.css";

// Hash routing keeps deep links working when served as static files by the
// daemon without any server-side route configuration.
const router = createHashRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Dashboard /> },
      { path: "timeline", element: <RunTimeline /> },
      { path: "timeline/:runId", element: <RunTimeline /> },
      { path: "patch", element: <PatchReview /> },
      { path: "patch/:runId", element: <PatchReview /> },
      { path: "cost", element: <CostCenter /> },
      { path: "cost/:runId", element: <CostCenter /> },
      { path: "risk", element: <RiskCenter /> },
      { path: "risk/:runId", element: <RiskCenter /> },
      { path: "rollback", element: <RollbackCenter /> },
      { path: "github", element: <GitHub /> },
      { path: "*", element: <NotFound /> },
    ],
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
