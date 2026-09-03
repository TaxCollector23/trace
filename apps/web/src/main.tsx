import React from "react";
import ReactDOM from "react-dom/client";
import { createHashRouter, RouterProvider } from "react-router-dom";
import App from "./App";
import ControlRoom from "./pages/ControlRoom";
import RunPage from "./pages/RunPage";
import SystemPage from "./pages/SystemPage";
import RunTimeline from "./pages/RunTimeline";
import PatchReview from "./pages/PatchReview";
import Usage from "./pages/Usage";
import RiskCenter from "./pages/RiskCenter";
import RollbackCenter from "./pages/RollbackCenter";
import GitHub from "./pages/GitHub";
import Ratify from "./pages/Ratify";
import Benchmarks from "./pages/Benchmarks";
import ComparePage from "./pages/ComparePage";
import NotFound from "./pages/NotFound";
import "./styles.css";

// Hash routing keeps deep links working when served as static files by the
// daemon without any server-side route configuration.
const router = createHashRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <ControlRoom /> },
      { path: "run", element: <RunPage /> },
      { path: "run/:runId", element: <RunPage /> },
      { path: "system", element: <SystemPage /> },
      { path: "timeline", element: <RunTimeline /> },
      { path: "timeline/:runId", element: <RunTimeline /> },
      { path: "patch", element: <PatchReview /> },
      { path: "patch/:runId", element: <PatchReview /> },
      { path: "usage", element: <Usage /> },
      { path: "usage/:runId", element: <Usage /> },
      { path: "risk", element: <RiskCenter /> },
      { path: "risk/:runId", element: <RiskCenter /> },
      { path: "benchmarks", element: <Benchmarks /> },
      { path: "compare", element: <ComparePage /> },
      { path: "rollback", element: <RollbackCenter /> },
      { path: "github", element: <GitHub /> },
      { path: "ratify", element: <Ratify /> },
      { path: "*", element: <NotFound /> },
    ],
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
