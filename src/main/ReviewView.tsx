import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { DomainWithSubs, EventWithClassifications } from "../lib/types";

interface Props {
  onChanged: () => void;
}

interface Draft {
  eventId: number;
  domains: { domainId: number; subdomainId: number | null }[];
}

export default function ReviewView({ onChanged }: Props) {
  const [events, setEvents] = useState<EventWithClassifications[]>([]);
  const [domains, setDomains] = useState<DomainWithSubs[]>([]);
  const [drafts, setDrafts] = useState<Map<number, Draft["domains"]>>(new Map());

  useEffect(() => {
    api.listEvents("needs_review", 500).then(setEvents);
    api.listDomains().then(setDomains);
  }, []);

  const toggleDomain = (eventId: number, domainId: number) => {
    setDrafts((prev) => {
      const next = new Map(prev);
      const cur = next.get(eventId) ?? [];
      const exists = cur.some((d) => d.domainId === domainId);
      next.set(
        eventId,
        exists
          ? cur.filter((d) => d.domainId !== domainId)
          : [...cur, { domainId, subdomainId: null }],
      );
      return next;
    });
  };

  const setSub = (eventId: number, domainId: number, subdomainId: number | null) => {
    setDrafts((prev) => {
      const next = new Map(prev);
      const cur = next.get(eventId) ?? [];
      next.set(
        eventId,
        cur.map((d) => (d.domainId === domainId ? { ...d, subdomainId } : d)),
      );
      return next;
    });
  };

  const save = async (event: EventWithClassifications) => {
    const draft = drafts.get(event.id);
    if (!draft || draft.length === 0) return;
    await api.review(
      event.id,
      draft.map((d) => d.domainId),
      draft.map((d) => d.subdomainId),
    );
    setDrafts((prev) => {
      const next = new Map(prev);
      next.delete(event.id);
      return next;
    });
    setEvents((prev) => prev.filter((e) => e.id !== event.id));
    onChanged();
  };

  if (events.length === 0) {
    return (
      <div className="view">
        <h2>Review queue</h2>
        <p className="muted">Nothing to review. Low-confidence entries land here for manual classification.</p>
      </div>
    );
  }

  return (
    <div className="view">
      <h2>Review queue</h2>
      {events.map((e) => {
        const draft = drafts.get(e.id) ?? [];
        return (
          <div key={e.id} className="review-card">
            <div className="review-top">
              <span className="recent-date">{e.event_date}</span>
              <span className="recent-text">{e.raw_text}</span>
            </div>
            <div className="review-domains">
              {domains.map((d) => {
                const active = draft.find((x) => x.domainId === d.id);
                return (
                  <div key={d.id} className="review-domain">
                    <label className="check">
                      <input
                        type="checkbox"
                        checked={!!active}
                        onChange={() => toggleDomain(e.id, d.id)}
                      />
                      <span style={{ color: d.color }}>{d.name}</span>
                    </label>
                    {active && (
                      <select
                        value={active.subdomainId ?? ""}
                        onChange={(ev) =>
                          setSub(
                            e.id,
                            d.id,
                            ev.target.value ? Number(ev.target.value) : null,
                          )
                        }
                      >
                        <option value="">— no subdomain —</option>
                        {d.subdomains.map((s) => (
                          <option key={s.id} value={s.id}>
                            {s.name}
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                );
              })}
            </div>
            <button
              className="save-btn"
              disabled={draft.length === 0}
              onClick={() => save(e)}
            >
              Save classification
            </button>
          </div>
        );
      })}
    </div>
  );
}