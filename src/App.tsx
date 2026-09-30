import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import CaptureApp from "./capture/CaptureApp";
import DashboardApp from "./main/DashboardApp";

export default function App() {
  // ponytail: label-based routing; URL params get re-encoded by the Tauri protocol handler
  const label = getCurrentWindow().label;
  useEffect(() => {
    invoke("diag", { msg: `window mounted: ${label}` }).catch(() => {});
  }, [label]);
  return label === "capture" ? <CaptureApp /> : <DashboardApp />;
}