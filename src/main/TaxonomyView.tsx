import { useEffect, useState } from "react";
import { api } from "../lib/api";
import type { DomainWithSubs } from "../lib/types";

interface Props {
  onChanged: () => void;
}

const PALETTE = ["#39d353", "#f78154", "#4c9fff", "#b78cfa", "#ffd166", "#06d6a0", "#ef476f", "#118ab2"];

interface EditState {
  name: string;
  description: string;
}

export default function TaxonomyView({ onChanged }: Props) {
  const [domains, setDomains] = useState<DomainWithSubs[]>([]);
  const [newDomain, setNewDomain] = useState("");
  const [newSub, setNewSub] = useState<Map<number, string>>(new Map());
  const [editing, setEditing] = useState<Map<number, EditState>>(new Map());

  const load = () => api.listDomains().then(setDomains);
  useEffect(() => {
    load();
  }, []);

  const addDomain = async () => {
    const name = newDomain.trim();
    if (!name) return;
    const color = PALETTE[domains.length % PALETTE.length];
    await api.addDomain(name, color, name);
    setNewDomain("");
    await load();
    onChanged();
  };

  const startEdit = (d: DomainWithSubs) =>
    setEditing((p) => new Map(p).set(d.id, { name: d.name, description: d.description }));

  const saveEdit = async (id: number) => {
    const e = editing.get(id);
    if (!e || !e.name.trim()) {
      setEditing((p) => {
        const n = new Map(p);
        n.delete(id);
        return n;
      });
      return;
    }
    await api.updateDomain(id, e.name.trim(), e.description.trim());
    setEditing((p) => {
      const n = new Map(p);
      n.delete(id);
      return n;
    });
    await load();
    onChanged();
  };

  const delDomain = async (id: number) => {
    if (!window.confirm("Delete domain and all its classifications?")) return;
    await api.deleteDomain(id);
    await load();
    onChanged();
  };

  const addSub = async (domainId: number) => {
    const name = (newSub.get(domainId) ?? "").trim();
    if (!name) return;
    await api.addSubdomain(domainId, name);
    setNewSub((p) => {
      const n = new Map(p);
      n.delete(domainId);
      return n;
    });
    await load();
  };

  const delSub = async (id: number) => {
    await api.deleteSubdomain(id);
    await load();
  };

  return (
    <div className="view">
      <h2>Taxonomy</h2>
      <p className="muted">
        Laya classifies only against these domains. The description is what Laya reads —
        write it like a definition ("Data structures and algorithms practice").
      </p>
      <div className="tax-add">
        <input
          placeholder="New domain name"
          value={newDomain}
          onChange={(e) => setNewDomain(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && addDomain()}
        />
        <button className="btn-primary" onClick={addDomain}>Add domain</button>
      </div>
      <div className="tax-grid">
      {domains.map((d) => {
        const edit = editing.get(d.id);
        return (
          <div key={d.id} className="tile tax-card" style={{ borderLeft: `3px solid ${d.color}` }}>
            <div className="tax-head">
              <span className="tax-dot" style={{ background: d.color }} />
              {edit ? (
                <input
                  value={edit.name}
                  onChange={(e) =>
                    setEditing((p) => new Map(p).set(d.id, { ...edit, name: e.target.value }))
                  }
                />
              ) : (
                <strong onDoubleClick={() => startEdit(d)} title="Double-click to edit">
                  {d.name}
                </strong>
              )}
              <button className="link-btn" onClick={() => delDomain(d.id)}>
                delete
              </button>
            </div>
            {edit ? (
              <div className="tax-desc-edit">
                <input
                  placeholder="Description Laya reads for classification"
                  value={edit.description}
                  onChange={(e) =>
                    setEditing((p) => new Map(p).set(d.id, { ...edit, description: e.target.value }))
                  }
                  onKeyDown={(e) => e.key === "Enter" && saveEdit(d.id)}
                />
                <button className="save-btn small" onClick={() => saveEdit(d.id)}>
                  Save
                </button>
              </div>
            ) : (
              d.description && <p className="tax-desc">{d.description}</p>
            )}
            <div className="tax-subs">
              {d.subdomains.map((s) => (
                <span key={s.id} className="sub-chip">
                  {s.name}
                  <button className="sub-x" onClick={() => delSub(s.id)}>
                    ×
                  </button>
                </span>
              ))}
              <input
                placeholder="+ subdomain"
                value={newSub.get(d.id) ?? ""}
                onChange={(e) => setNewSub((p) => new Map(p).set(d.id, e.target.value))}
                onKeyDown={(e) => e.key === "Enter" && addSub(d.id)}
              />
            </div>
          </div>
        );
      })}
      </div>
    </div>
  );
}