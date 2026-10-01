use std::path::Path;

use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::{
    Classification, DayCount, Domain, DomainWithSubs, Event, EventWithClassifications, NewDomain,
    NewSubdomain, Subdomain,
};

pub struct Db(pub Connection);

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.pragma_update(None, "foreign_keys", "ON").ok();
        let db = Db(conn);
        db.migrate()?;
        db.seed_default_taxonomy()?;
        Ok(db)
    }

    fn migrate(&self) -> rusqlite::Result<()> {
        let v: i64 = self
            .0
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if v < 1 {
            self.0.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS domains (
                    id INTEGER PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    color TEXT NOT NULL DEFAULT '#39d353',
                    description TEXT NOT NULL DEFAULT '',
                    sort INTEGER NOT NULL DEFAULT 0
                );
                CREATE TABLE IF NOT EXISTS subdomains (
                    id INTEGER PRIMARY KEY,
                    domain_id INTEGER NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    sort INTEGER NOT NULL DEFAULT 0,
                    UNIQUE(domain_id, name)
                );
                CREATE TABLE IF NOT EXISTS events (
                    id INTEGER PRIMARY KEY,
                    raw_text TEXT NOT NULL,
                    canonical_topic TEXT NOT NULL,
                    event_date TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    confidence REAL NOT NULL DEFAULT 0,
                    source TEXT NOT NULL DEFAULT 'hotkey',
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_events_date ON events(event_date);
                CREATE INDEX IF NOT EXISTS idx_events_status ON events(status);
                CREATE TABLE IF NOT EXISTS classifications (
                    id INTEGER PRIMARY KEY,
                    event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE,
                    domain_id INTEGER NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
                    subdomain_id INTEGER REFERENCES subdomains(id) ON DELETE CASCADE,
                    probability REAL NOT NULL DEFAULT 0,
                    UNIQUE(event_id, domain_id, subdomain_id)
                );
                CREATE INDEX IF NOT EXISTS idx_class_event ON classifications(event_id);
                CREATE INDEX IF NOT EXISTS idx_class_domain ON classifications(domain_id);
                CREATE TABLE IF NOT EXISTS settings (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                PRAGMA user_version = 1;
                ",
            )?;
        }
        if v < 2 {
            // v2: remove duplicate classification rows left by the pre-claim
            // race (one row per event+domain survives, best probability),
            // and tighten the auto-save gate.
            self.0.execute_batch(
                "
                DELETE FROM classifications
                WHERE id NOT IN (
                    SELECT c.id FROM classifications c
                    JOIN (
                        SELECT event_id, domain_id, MAX(probability) AS mp, MIN(id) AS mid
                        FROM classifications GROUP BY event_id, domain_id
                    ) k ON c.event_id = k.event_id
                       AND c.domain_id = k.domain_id
                       AND c.probability = k.mp
                       AND c.id = k.mid
                );
                UPDATE settings SET value = '0.5' WHERE key = 'review_threshold' AND value = '0.35';
                PRAGMA user_version = 2;
                ",
            )?;
        }
        if v < 3 {
            // v3: hotkey moved from Ctrl+Shift+Space to Ctrl+`
            self.0.execute_batch(
                "
                UPDATE settings SET value = 'Ctrl+`' WHERE key = 'hotkey' AND value = 'Ctrl+Shift+Space';
                PRAGMA user_version = 3;
                ",
            )?;
        }
        Ok(())
    }

    fn seed_default_taxonomy(&self) -> rusqlite::Result<()> {
        let count: i64 = self.0.query_row("SELECT COUNT(*) FROM domains", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }
        let seeds: Vec<(String, String, String, Vec<&str>)> = vec![
            (
                "DSA".into(),
                "#39d353".into(),
                "Data structures and algorithms practice".into(),
                vec!["Arrays", "Strings", "Linked Lists", "Trees", "Graphs", "DP", "Hashing"],
            ),
            (
                "System Design".into(),
                "#f78154".into(),
                "System design, architecture, distributed systems, backend infrastructure".into(),
                vec!["HLD", "LLD", "Distributed Systems", "Databases", "Caching", "Message Queues"],
            ),
            (
                "Web Development".into(),
                "#4c9fff".into(),
                "Web development: frontend, backend, databases, devops".into(),
                vec!["Frontend", "Backend", "Database", "DevOps"],
            ),
            (
                "AI/ML".into(),
                "#b78cfa".into(),
                "Machine learning and AI: LLMs, RAG, agents, model training".into(),
                vec!["LLM", "RAG", "Agents", "ML"],
            ),
            (
                "Aptitude".into(),
                "#ffd166".into(),
                "Quantitative, logical or verbal reasoning practice".into(),
                vec!["Quantitative", "Logical Reasoning", "Verbal"],
            ),
            (
                "Exercise".into(),
                "#06d6a0".into(),
                "Physical exercise or fitness".into(),
                vec!["Strength", "Cardio", "Mobility"],
            ),
        ];
        for (sort, (name, color, description, subs)) in seeds.into_iter().enumerate() {
            self.0.execute(
                "INSERT INTO domains (name, color, description, sort) VALUES (?1, ?2, ?3, ?4)",
                params![name, color, description, sort as i64],
            )?;
            let domain_id = self.0.last_insert_rowid();
            for (s, sub) in subs.into_iter().enumerate() {
                self.0.execute(
                    "INSERT INTO subdomains (domain_id, name, sort) VALUES (?1, ?2, ?3)",
                    params![domain_id, sub, s as i64],
                )?;
            }
        }
        let defaults: Vec<(&str, &str)> = vec![
            ("hotkey", "Ctrl+`"),
            ("laya_url", "http://127.0.0.1:8080"),
            ("multi_ratio", "0.5"),
            ("multi_floor", "0.2"),
            ("subdomain_threshold", "0.3"),
            ("review_threshold", "0.5"),
            ("start_at_login", "false"),
        ];
        for (k, v) in defaults {
            self.0.execute(
                "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
                params![k, v],
            )?;
        }
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> String {
        self.0
            .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| r.get(0))
            .unwrap_or_default()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting_f64(&self, key: &str, default: f64) -> f64 {
        self.get_setting(key).parse::<f64>().unwrap_or(default)
    }

    pub fn list_domains(&self) -> rusqlite::Result<Vec<DomainWithSubs>> {
        let mut stmt = self
            .0
            .prepare("SELECT id, name, color, description, sort FROM domains ORDER BY sort, id")?;
        let domains: Vec<Domain> = stmt
            .query_map([], |r| {
                Ok(Domain {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                    description: r.get(3)?,
                    sort: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::new();
        for d in domains {
            let mut s = self
                .0
                .prepare("SELECT id, domain_id, name, sort FROM subdomains WHERE domain_id = ?1 ORDER BY sort, id")?;
            let subs: Vec<Subdomain> = s
                .query_map(params![d.id], |r| {
                    Ok(Subdomain {
                        id: r.get(0)?,
                        domain_id: r.get(1)?,
                        name: r.get(2)?,
                        sort: r.get(3)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?;
            out.push(DomainWithSubs { domain: d, subdomains: subs });
        }
        Ok(out)
    }

    pub fn add_domain(&self, nd: &NewDomain) -> rusqlite::Result<i64> {
        let sort: i64 = self
            .0
            .query_row("SELECT COALESCE(MAX(sort), -1) + 1 FROM domains", [], |r| r.get(0))?;
        self.0.execute(
            "INSERT INTO domains (name, color, description, sort) VALUES (?1, ?2, ?3, ?4)",
            params![nd.name, nd.color, nd.description, sort],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn update_domain(&self, id: i64, name: &str, description: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE domains SET name = ?2, description = ?3 WHERE id = ?1",
            params![id, name, description],
        )?;
        Ok(())
    }

    pub fn delete_domain(&self, id: i64) -> rusqlite::Result<()> {
        self.0
            .execute("DELETE FROM domains WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn add_subdomain(&self, ns: &NewSubdomain) -> rusqlite::Result<i64> {
        let sort: i64 = self.0.query_row(
            "SELECT COALESCE(MAX(sort), -1) + 1 FROM subdomains WHERE domain_id = ?1",
            params![ns.domain_id],
            |r| r.get(0),
        )?;
        self.0.execute(
            "INSERT INTO subdomains (domain_id, name, sort) VALUES (?1, ?2, ?3)",
            params![ns.domain_id, ns.name, sort],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn delete_subdomain(&self, id: i64) -> rusqlite::Result<()> {
        self.0
            .execute("DELETE FROM subdomains WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn insert_event(
        &self,
        raw_text: &str,
        canonical_topic: &str,
        event_date: &str,
        source: &str,
    ) -> rusqlite::Result<i64> {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.0.execute(
            "INSERT INTO events (raw_text, canonical_topic, event_date, status, source, created_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5)",
            params![raw_text, canonical_topic, event_date, source, now],
        )?;
        Ok(self.0.last_insert_rowid())
    }

    pub fn set_event_status(&self, id: i64, status: &str, confidence: f64) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE events SET status = ?2, confidence = ?3 WHERE id = ?1",
            params![id, status, confidence],
        )?;
        Ok(())
    }

    /// Atomically claim a pending event for classification. Returns false if
    /// another worker already claimed or classified it — prevents the sync
    /// capture path and the retry loop from double-classifying.
    pub fn claim_event(&self, id: i64) -> rusqlite::Result<bool> {
        let n = self.0.execute(
            "UPDATE events SET status = 'classifying' WHERE id = ?1 AND status = 'pending'",
            params![id],
        )?;
        Ok(n == 1)
    }

    /// Mark an event as manually classified in the review queue.
    pub fn mark_reviewed(&self, id: i64) -> rusqlite::Result<()> {
        self.0.execute(
            "UPDATE events SET status = 'auto_saved', confidence = 1.0, source = 'review' WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Raw text + domain names of the most recent user corrections (source='review'),
    /// used as few-shot examples for classification.
    pub fn recent_corrections(&self, limit: i64) -> rusqlite::Result<Vec<(String, String)>> {
        let mut stmt = self.0.prepare(
            "SELECT e.raw_text, GROUP_CONCAT(d.name, ', ')
             FROM events e
             JOIN classifications c ON c.event_id = e.id
             JOIN domains d ON d.id = c.domain_id
             WHERE e.source = 'review'
             GROUP BY e.id ORDER BY e.id DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(|e| e)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn clear_classifications(&self, event_id: i64) -> rusqlite::Result<()> {
        self.0
            .execute("DELETE FROM classifications WHERE event_id = ?1", params![event_id])?;
        Ok(())
    }

    pub fn insert_classification(
        &self,
        event_id: i64,
        domain_id: i64,
        subdomain_id: Option<i64>,
        probability: f64,
    ) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT OR REPLACE INTO classifications (event_id, domain_id, subdomain_id, probability)
             VALUES (?1, ?2, ?3, ?4)",
            params![event_id, domain_id, subdomain_id, probability],
        )?;
        Ok(())
    }

    pub fn pending_events(&self, limit: i64) -> rusqlite::Result<Vec<i64>> {
        let mut stmt = self
            .0
            .prepare("SELECT id FROM events WHERE status = 'pending' ORDER BY id LIMIT ?1")?;
        let ids = stmt
            .query_map(params![limit], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        Ok(ids)
    }

    /// Permanently delete an event and its classifications (FK cascade).
    pub fn delete_event(&self, id: i64) -> rusqlite::Result<()> {
        self.0.execute("DELETE FROM events WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_event(&self, id: i64) -> rusqlite::Result<Event> {
        self.0
            .query_row(
                "SELECT id, raw_text, canonical_topic, event_date, status, confidence, source, created_at
                 FROM events WHERE id = ?1",
                params![id],
                |r| {
                    Ok(Event {
                        id: r.get(0)?,
                        raw_text: r.get(1)?,
                        canonical_topic: r.get(2)?,
                        event_date: r.get(3)?,
                        status: r.get(4)?,
                        confidence: r.get(5)?,
                        source: r.get(6)?,
                        created_at: r.get(7)?,
                    })
                },
            )
            .optional()
            .map(|o| o.expect("event exists"))
    }

    pub fn list_events(
        &self,
        status: Option<&str>,
        domain_id: Option<i64>,
        limit: i64,
    ) -> rusqlite::Result<Vec<EventWithClassifications>> {
        let mut sql = String::from(
            "SELECT id, raw_text, canonical_topic, event_date, status, confidence, source, created_at
             FROM events WHERE 1=1",
        );
        let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = status {
            sql.push_str(&format!(" AND status = ?{}", args.len() + 1));
            args.push(Box::new(s.to_string()));
        }
        if let Some(d) = domain_id {
            sql.push_str(&format!(
                " AND id IN (SELECT event_id FROM classifications WHERE domain_id = ?{})",
                args.len() + 1
            ));
            args.push(Box::new(d));
        }
        sql.push_str(" ORDER BY event_date DESC, id DESC");
        sql.push_str(&format!(" LIMIT {limit}"));
        let mut stmt = self.0.prepare(&sql)?;
        let events: Vec<Event> = stmt
            .query_map(
                rusqlite::params_from_iter(args.iter().map(|a| a.as_ref())),
                |r| self.row_to_event(r),
            )?
            .collect::<rusqlite::Result<_>>()?;
        let mut out = Vec::new();
        for e in events {
            out.push(EventWithClassifications {
                classifications: self.event_classifications(e.id)?,
                event: e,
            });
        }
        Ok(out)
    }

    fn row_to_event(&self, r: &rusqlite::Row) -> rusqlite::Result<Event> {
        Ok(Event {
            id: r.get(0)?,
            raw_text: r.get(1)?,
            canonical_topic: r.get(2)?,
            event_date: r.get(3)?,
            status: r.get(4)?,
            confidence: r.get(5)?,
            source: r.get(6)?,
            created_at: r.get(7)?,
        })
    }

    pub fn event_classifications(&self, event_id: i64) -> rusqlite::Result<Vec<Classification>> {
        let mut stmt = self.0.prepare(
            "SELECT c.domain_id, d.name, c.subdomain_id, s.name, c.probability
             FROM classifications c
             JOIN domains d ON d.id = c.domain_id
             LEFT JOIN subdomains s ON s.id = c.subdomain_id
             WHERE c.event_id = ?1 ORDER BY c.probability DESC",
        )?;
        let rows = stmt
            .query_map(params![event_id], |r| {
                Ok(Classification {
                    domain_id: r.get(0)?,
                    domain_name: r.get(1)?,
                    subdomain_id: r.get(2)?,
                    subdomain_name: r.get(3)?,
                    probability: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn daily_counts(&self, domain_id: Option<i64>, from: &str, to: &str) -> rusqlite::Result<Vec<DayCount>> {
        let sql = match domain_id {
            Some(_) => "SELECT e.event_date, COUNT(DISTINCT e.canonical_topic)
                        FROM events e
                        JOIN classifications c ON c.event_id = e.id
                        WHERE e.event_date BETWEEN ?1 AND ?2 AND c.domain_id = ?3
                        GROUP BY e.event_date",
            None => "SELECT e.event_date, COUNT(DISTINCT e.canonical_topic)
                     FROM events e
                     JOIN classifications c ON c.event_id = e.id
                     WHERE e.event_date BETWEEN ?1 AND ?2
                     GROUP BY e.event_date",
        };
        let mut stmt = self.0.prepare(sql)?;
        let rows = match domain_id {
            Some(did) => stmt
                .query_map(params![from, to, did], |r| {
                    Ok(DayCount { date: r.get(0)?, count: r.get(1)? })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?,
            None => stmt
                .query_map(params![from, to], |r| {
                    Ok(DayCount { date: r.get(0)?, count: r.get(1)? })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?,
        };
        Ok(rows)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{NewDomain, NewSubdomain};

    fn temp_db() -> (Db, std::path::PathBuf) {
        // unique per call: parallel tests must not share one SQLite file
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("learngraph-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);
        (Db::open(&path).unwrap(), path)
    }

    #[test]
    fn seeds_and_lists_taxonomy() {
        let (db, _p) = temp_db();
        let domains = db.list_domains().unwrap();
        assert_eq!(domains.len(), 6);
        assert!(domains[0].subdomains.len() > 0);
    }

    #[test]
    fn daily_counts_dedupes_topics_and_splits_domains() {
        let (db, _p) = temp_db();
        let domains = db.list_domains().unwrap();
        let dsa = &domains[0];
        let web = &domains[1];

        let e1 = db.insert_event("2sum", "2sum", "2026-09-28", "test").unwrap();
        let e2 = db.insert_event("2sum", "2sum", "2026-09-28", "test").unwrap();
        let e3 = db.insert_event("sliding window", "sliding window", "2026-09-28", "test").unwrap();

        // same topic twice, both domains: distinct-topic count must stay 1 for that topic
        for (e, d) in [(e1, dsa.domain.id), (e2, dsa.domain.id), (e3, dsa.domain.id), (e2, web.domain.id)] {
            db.insert_classification(e, d, None, 0.9).unwrap();
        }
        db.set_event_status(e1, "auto_saved", 0.9).unwrap();
        db.set_event_status(e2, "auto_saved", 0.9).unwrap();
        db.set_event_status(e3, "auto_saved", 0.9).unwrap();

        let dsa_counts = db.daily_counts(Some(dsa.domain.id), "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(dsa_counts.len(), 1);
        assert_eq!(dsa_counts[0].count, 2); // 2sum + sliding window (2sum deduped)

        let web_counts = db.daily_counts(Some(web.domain.id), "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(web_counts[0].count, 1); // only via e2

        let overall = db.daily_counts(None, "2026-09-01", "2026-09-30").unwrap();
        assert_eq!(overall[0].count, 2);

        // list_events domain filter: only entries classified into the domain
        let dsa_events = db.list_events(None, Some(dsa.domain.id), 10).unwrap();
        assert_eq!(dsa_events.len(), 3);
        let web_events = db.list_events(None, Some(web.domain.id), 10).unwrap();
        assert_eq!(web_events.len(), 1);
    }

    #[test]
    fn pending_and_status_flow() {
        let (db, _p) = temp_db();
        let id = db.insert_event("jwt refresh", "jwt refresh", "2026-09-29", "hotkey").unwrap();
        assert_eq!(db.pending_events(10).unwrap(), vec![id]);
        db.set_event_status(id, "needs_review", 0.2).unwrap();
        assert!(db.pending_events(10).unwrap().is_empty());
        let events = db.list_events(Some("needs_review"), None, 10).unwrap();
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn taxonomy_crud_and_validation() {
        let (db, _p) = temp_db();
        let domains = db.list_domains().unwrap();
        let dsa_id = domains[0].domain.id;
        let sub_id = domains[0].subdomains[0].id;

        assert!(crate::laya::valid_classification(&db, dsa_id, Some(sub_id)));
        assert!(crate::laya::valid_classification(&db, dsa_id, None));
        assert!(!crate::laya::valid_classification(&db, 99999, None));
        assert!(!crate::laya::valid_classification(&db, dsa_id, Some(99999)));

        let new_id = db
            .add_domain(&NewDomain {
                name: "Test".into(),
                color: "#fff".into(),
                description: "test domain".into(),
            })
            .unwrap();
        let sub = db.add_subdomain(&NewSubdomain { domain_id: new_id, name: "Sub".into() }).unwrap();
        assert!(crate::laya::valid_classification(&db, new_id, Some(sub)));
        db.delete_subdomain(sub).unwrap();
        assert!(!crate::laya::valid_classification(&db, new_id, Some(sub)));
        db.delete_domain(new_id).unwrap();
    }
}