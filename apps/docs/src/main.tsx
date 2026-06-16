import React from "react";
import ReactDOM from "react-dom/client";
import { createHashRouter, RouterProvider, Navigate } from "react-router-dom";
import App from "./App";
import Doc from "./Doc";
import { orderedSlugs } from "./content";
import "highlight.js/styles/github-dark.css";
import "./styles.css";

const first = orderedSlugs()[0] ?? "overview";

const router = createHashRouter([
  {
    path: "/",
    element: <App />,
    children: [
      { index: true, element: <Navigate to={`/${first}`} replace /> },
      { path: "*", element: <Doc /> },
    ],
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <RouterProvider router={router} />
  </React.StrictMode>
);
