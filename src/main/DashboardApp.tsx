import { useCallback, useEffect, useState } from "react";
import Heatmap, { PROJECT_START } from "../components/Heatmap";
import { api } from "../lib/api";
import type { DayCount, DomainWithSubs, EventWithClassifications } from "../lib/types";
import ReviewView from "./ReviewView";
import SettingsView from "./SettingsView";
import TaxonomyView from "./TaxonomyView";

type Tab = "heatmaps" | "review" | "taxonomy" | "settings";

function iso(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(
    d.getDate(),
  ).padStart(2, "0")}`;
}

export default function DashboardApp() {
  const [tab, setTab] = useState<Tab>("heatmaps");
  const [domains, setDomains] = useState<DomainWithSubs[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [counts, setCounts] = useState<DayCount[]>([]);
  const [recent, setRecent] = useState<EventWithClassifications[]>([]);
  const [daemonUp, setDaemonUp] = useState<boolean | null>(null);
  const [reviewCount, setReviewCount] = useState(0);
  const [refreshKey, setRefreshKey] = useState(0);

  const refresh = useCallback(() => setRefreshKey((k) => k + 1), []);

  // startup race: the webview can invoke before Tauri state is managed;
  // those failures are transient, so retry once shortly after
  const retrySoon = useCallback(
    (err: unknown) => {
      if (String(err).includes("state not managed")) setTimeout(refresh, 300);
    },
    [refresh],
  );

  useEffect(() => {
    api.daemonHealth().then(
      (h) => setDaemonUp(h.status === "ok"),
      (e) => {
        setDaemonUp(false);
        retrySoon(e);
      },
    );
  }, [refreshKey, retrySoon]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [ds, events] = await Promise.all([
          api.listDomains(),
          api.listEvents("needs_review", 500),
        ]);
        if (cancelled) return;
        setDomains(ds);
        setReviewCount(events.length);
      } catch (e) {
        retrySoon(e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [refreshKey, retrySoon]);

  useEffect(() => {
    let cancelled = false;
    api
      .dailyCounts(selected, PROJECT_START, iso(new Date()))
      .then((c) => {
        if (!cancelled) setCounts(c);
      })
      .catch((e) => retrySoon(e));
    api
      .listEvents(null, 8, selected)
      .then((e) => {
        if (!cancelled) setRecent(e);
      })
      .catch((e) => retrySoon(e));
    return () => {
      cancelled = true;
    };
  }, [selected, refreshKey, retrySoon]);

  const selectedDomain = domains.find((d) => d.id === selected) ?? null;

  const weekAgo = iso(new Date(Date.now() - 6 * 86400000));
  const total = counts.reduce((s, c) => s + c.count, 0);
  const thisWeek = counts
    .filter((c) => c.date >= weekAgo)
    .reduce((s, c) => s + c.count, 0);

  const reclassify = async (id: number) => {
    try {
      await api.reclassify(id);
    } finally {
      // success or daemon outage (event back to pending) — reflect it
      refresh();
    }
  };

  return (
    <div className="dashboard">
      <header className="dash-header">
        <h1>LearnGraph</h1>
        <span className={`daemon-dot ${daemonUp ? "up" : "down"}`} title={daemonUp ? "Laya daemon online" : "Laya daemon offline"}>
          {daemonUp ? "daemon online" : "daemon offline"}
        </span>
        <button className="refresh-btn" onClick={refresh} title="Reload data and daemon status">
          Refresh
        </button>
      </header>
      <nav className="tabs">
        {(
          [
            ["heatmaps", "Heatmaps"],
            ["review", `Review${reviewCount ? ` (${reviewCount})` : ""}`],
            ["taxonomy", "Taxonomy"],
            ["settings", "Settings"],
          ] as [Tab, string][]
        ).map(([id, label]) => (
          <button
            key={id}
            className={tab === id ? "tab active" : "tab"}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </nav>

      {tab === "heatmaps" && (
        <div className="view">
          <div className="domain-chips">
            <button
              className={selected === null ? "chip active" : "chip"}
              onClick={() => setSelected(null)}
            >
              Overall
            </button>
            {domains.map((d) => (
              <button
                key={d.id}
                className={selected === d.id ? "chip active" : "chip"}
                onClick={() => setSelected(d.id)}
              >
                {d.name}
              </button>
            ))}
          </div>
          <div className="bento">
            <div className="tile stat">
              <small>Total entries</small>
              <strong>{total}</strong>
            </div>
            <div className="tile stat">
              <small>This week</small>
              <strong>{thisWeek}</strong>
            </div>
            <div className="tile stat">
              <small>Needs review</small>
              <strong className={reviewCount > 0 ? "warn" : undefined}>{reviewCount}</strong>
            </div>
            <div className="tile stat">
              <small>Daemon</small>
              <strong className={daemonUp ? undefined : "warn"}>
                {daemonUp ? "online" : "offline"}
              </strong>
            </div>
            <section className="tile wide">
              <div className="tile-head">{selectedDomain ? selectedDomain.name : "Overall"}</div>
              <Heatmap counts={counts} color={selectedDomain?.color ?? "#39d353"} />
            </section>
            <section className="tile wide">
              <div className="tile-head">Recent entries</div>
              {recent.length === 0 && (
                <p className="muted">No entries yet. Press Ctrl+` to log your first one.</p>
              )}
              {recent.map((e) => (
                <div key={e.id} className="recent-row">
                  <span className="recent-date">{e.event_date}</span>
                  <span className="recent-text">{e.raw_text}</span>
                  <span className="recent-tags">
                    {e.classifications.map((c, i) => (
                      <span key={`${c.domain_id}-${i}`} className="tag">
                        {c.domain_name}
                        {c.subdomain_name ? ` · ${c.subdomain_name}` : ""}
                      </span>
                    ))}
                    {e.classifications.length === 0 && (
                      <span className="tag pending">{e.status === "pending" ? "pending" : "unclassified"}</span>
                    )}
                  </span>
                  <button className="link-btn reclassify" onClick={() => reclassify(e.id)} title="Re-run AI classification">
                    reclassify
                  </button>
                </div>
              ))}
            </section>
          </div>
        </div>
      )}

      {tab === "review" && <ReviewView key={refreshKey} onChanged={refresh} />}
      {tab === "taxonomy" && <TaxonomyView key={refreshKey} onChanged={refresh} />}
      {tab === "settings" && <SettingsView key={refreshKey} />}
    </div>
  );
}