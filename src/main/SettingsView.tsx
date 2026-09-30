import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { Settings } from "../lib/types";

const LABELS: Record<string, string> = {
  hotkey: "Global hotkey",
  laya_url: "Laya daemon URL",
  domain_threshold: "Domain threshold (primary)",
  multi_ratio: "Multi-domain ratio",
  multi_floor: "Multi-domain floor",
  review_threshold: "Auto-save threshold",
  subdomain_threshold: "Subdomain threshold",
  start_at_login: "Start with Windows",
};

const HELP: Record<string, string> = {
  domain_threshold: "Minimum probability for the primary domain to be accepted.",
  multi_ratio: "Secondary domain must score at least this × the winner.",
  multi_floor: "Absolute minimum for any secondary domain.",
  review_threshold: "Below this, the entry goes to the review queue.",
  subdomain_threshold: "Minimum probability for a subdomain pick.",
};

export default function SettingsView() {
  const [settings, setSettings] = useState<Settings>({});
  const [health, setHealth] = useState<string>("checking…");

  useEffect(() => {
    api.getSettings().then((s) => {
      const map: Settings = {};
      for (const [k, v] of s) map[k] = v;
      setSettings(map);
    });
    api.daemonHealth().then(
      (h) => setHealth(`daemon online (${h.device ?? "cpu"})`),
      () => setHealth("daemon offline — entries will queue as pending"),
    );
  }, []);

  const update = (key: string, value: string) => {
    setSettings((p) => ({ ...p, [key]: value }));
    api.setSetting(key, value);
  };

  const numeric = (key: string, min: number, max: number, step: number) => (
    <input
      type="number"
      min={min}
      max={max}
      step={step}
      value={settings[key] ?? ""}
      onChange={(e) => update(key, e.target.value)}
    />
  );

  return (
    <div className="view">
      <h2>Settings</h2>
      <p className="muted">{health}</p>
      <div className="settings-grid">
        {Object.keys(settings)
          .filter((k) => k !== "start_at_login")
          .map((k) => (
            <label key={k} className="setting-row">
              <span>
                {LABELS[k] ?? k}
                {HELP[k] && <small>{HELP[k]}</small>}
              </span>
              {k === "laya_url" ? (
                <input
                  value={settings[k]}
                  onChange={(e) => update(k, e.target.value)}
                />
              ) : k === "hotkey" ? (
                <input value={settings[k]} onChange={(e) => update(k, e.target.value)} disabled title="Fixed in this version" />
              ) : (
                numeric(k, 0, 1, 0.05)
              )}
            </label>
          ))}
        <label className="setting-row">
          <span>{LABELS["start_at_login"]}</span>
          <input
            type="checkbox"
            checked={settings["start_at_login"] === "true"}
            onChange={(e) => {
              const v = String(e.target.checked);
              update("start_at_login", v);
              api.setSetting("start_at_login", v).then(() => {
                // ponytail: autostart is wired via the plugin; apply immediately.
                import("@tauri-apps/plugin-autostart").then((m) => {
                  if (e.target.checked) m.enable();
                  else m.disable();
                });
              });
            }}
          />
        </label>
      </div>
    </div>
  );
}