import React from "react";
import ReactDOM from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";
import App from "./App";
import Home from "./pages/Home";
import About from "./pages/About";
import DesktopDownload from "./pages/DesktopDownload";
import CliDownload from "./pages/CliDownload";
import HostedDashboard from "./pages/HostedDashboard";
import Private from "./pages/Private";
import "./styles.css";

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Home /> },
      { path: "about", element: <About /> },
      { path: "download", element: <DesktopDownload /> },
      { path: "cli", element: <CliDownload /> },
      { path: "dashboard", element: <HostedDashboard /> },
      { path: "private", element: <Private /> },
    ],
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
