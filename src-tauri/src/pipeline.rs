use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::Db;
use crate::laya;
use crate::models::{CaptureRequest, CaptureResult, ReviewUpdate};

pub struct AppState {
    pub db: Mutex<Db>,
    pub db_path: PathBuf,
}

pub const EVENT_CAPTURED: &str = "event:captured";
pub const EVENT_CLASSIFIED: &str = "event:classified";

/// Persist-first: the raw text is written to SQLite before anything else.
/// Classification runs without holding the DB lock; a daemon outage leaves the
/// event pending and the background loop retries it.
pub fn capture(state: &AppState, req: &CaptureRequest) -> Result<CaptureResult, String> {
    let text = req.text.trim().to_string();
    if text.is_empty() {
        return Err("empty capture".into());
    }
    let date = req
        .date
        .clone()
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());
    let topic = laya::canonical_topic(&text);
    if topic.is_empty() {
        return Err("empty after normalization".into());
    }
    let event_id = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.insert_event(&text, &topic, &date, "hotkey")
            .map_err(|e| e.to_string())?
    };
    let result = classify_pending(&state.db, event_id)?;
    Ok(result)
}

/// Re-run classification on an existing event (user disagreed with the model).
/// Resets to pending so claim_event succeeds; if the daemon is unreachable the
/// event stays pending for the retry loop instead of being stuck in review.
pub fn reclassify(state: &AppState, event_id: i64) -> Result<CaptureResult, String> {
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        db.set_event_status(event_id, "pending", 0.0)
            .map_err(|e| e.to_string())?;
    }
    classify_pending(&state.db, event_id)
}

/// Classify one event: claim it atomically (pending -> classifying), snapshot
/// ctx (short lock), HTTP calls (no lock), write outcome (short lock).
/// Returns the final status; Err means the daemon was unreachable and the
/// event goes back to pending for the retry loop.
fn classify_pending(db_lock: &Mutex<Db>, event_id: i64) -> Result<CaptureResult, String> {
    let (claimed, text, ctx) = {
        let db = db_lock.lock().map_err(|e| e.to_string())?;
        let claimed = db.claim_event(event_id).map_err(|e| e.to_string())?;
        let text = db.get_event(event_id).map_err(|e| e.to_string())?.raw_text;
        let ctx = laya::build_ctx(&db)?;
        (claimed, text, ctx)
    };
    if !claimed {
        // already classified or claimed by the other worker
        let db = db_lock.lock().map_err(|e| e.to_string())?;
        let status = db.get_event(event_id).map_err(|e| e.to_string())?.status;
        return Ok(CaptureResult { event_id, status });
    }
    let outcome = match laya::classify(&ctx, &text) {
        Ok(o) => o,
        Err(e) => {
            let db = db_lock.lock().map_err(|e2| e2.to_string())?;
            db.set_event_status(event_id, "pending", 0.0).map_err(|e2| e2.to_string())?;
            return Err(e);
        }
    };
    let db = db_lock.lock().map_err(|e| e.to_string())?;
    db.clear_classifications(event_id).map_err(|e| e.to_string())?;
    // one row per domain: keep the strongest classification
    let mut best: std::collections::HashMap<i64, &crate::models::Classification> =
        std::collections::HashMap::new();
    for c in &outcome.classifications {
        match best.get(&c.domain_id) {
            Some(prev) if prev.probability >= c.probability => {}
            _ => {
                best.insert(c.domain_id, c);
            }
        }
    }
    for c in best.values() {
        db.insert_classification(event_id, c.domain_id, c.subdomain_id, c.probability)
            .map_err(|e| e.to_string())?;
    }
    db.set_event_status(event_id, &outcome.status, outcome.confidence)
        .map_err(|e| e.to_string())?;
    Ok(CaptureResult {
        event_id,
        status: outcome.status,
    })
}

/// Apply a manual classification from the review queue (deterministic, validated).
pub fn apply_review(
    state: &AppState,
    app: &AppHandle,
    update: &ReviewUpdate,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    if update.domain_ids.is_empty() {
        return Err("at least one domain required".into());
    }
    if update.subdomain_ids.len() != update.domain_ids.len() {
        return Err("subdomain list length mismatch".into());
    }
    // dedupe repeated domain picks; keep the first (domain, subdomain) pair
    let mut seen = std::collections::HashSet::new();
    let mut pairs: Vec<(i64, Option<i64>)> = Vec::new();
    for (i, domain_id) in update.domain_ids.iter().enumerate() {
        if seen.insert(*domain_id) {
            pairs.push((*domain_id, update.subdomain_ids[i]));
        }
    }
    for (domain_id, sub) in &pairs {
        if !laya::valid_classification(&db, *domain_id, *sub) {
            return Err(format!("invalid taxonomy ids: domain {domain_id} sub {sub:?}"));
        }
    }
    // user's picks replace the model's, not merge with them
    db.clear_classifications(update.event_id).map_err(|e| e.to_string())?;
    for (domain_id, sub) in &pairs {
        db.insert_classification(update.event_id, *domain_id, *sub, 1.0)
            .map_err(|e| e.to_string())?;
    }
    db.mark_reviewed(update.event_id).map_err(|e| e.to_string())?;
    let _ = app.emit(EVENT_CLASSIFIED, update.event_id);
    Ok(())
}

/// Background loop: retry pending events while the daemon is down.
/// Owns its own SQLite connection so the UI connection lock is never held
/// across HTTP calls.
pub fn spawn_retry_loop(app: AppHandle, db_path: PathBuf) {
    tauri::async_runtime::spawn(async move {
        let db = match Db::open(&db_path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let db_lock = Mutex::new(db);
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let pending = match db_lock.lock() {
                Ok(db) => match db.pending_events(50) {
                    Ok(p) => p,
                    Err(_) => continue,
                },
                Err(_) => continue,
            };
            if pending.is_empty() {
                continue;
            }
            let mut classified: Vec<i64> = Vec::new();
            for id in pending {
                match classify_pending(&db_lock, id) {
                    Ok(_) => classified.push(id),
                    Err(_) => break, // daemon still down; give it another cycle
                }
            }
            for id in classified {
                let _ = app.emit(EVENT_CLASSIFIED, id);
            }
        }
    });
}

pub fn show_capture_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.center();
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CaptureRequest;

    fn temp_state() -> (PathBuf, AppState) {
        let dir = std::env::temp_dir().join(format!("learngraph-pipe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);
        (
            path.clone(),
            AppState {
                db: Mutex::new(Db::open(&path).unwrap()),
                db_path: path,
            },
        )
    }

    /// Persist-first smoke: capture either classifies (daemon up) or stays
    /// pending (daemon down) - the raw event must exist in both cases.
    #[test]
    fn capture_persists_before_classification() {
        let (_p, state) = temp_state();
        let req = CaptureRequest {
            text: "solved Two Sum with a hash map".into(),
            date: None,
        };
        match capture(&state, &req) {
            Ok(res) => {
                assert!(res.status == "auto_saved" || res.status == "needs_review");
                let db = state.db.lock().unwrap();
                let ev = db.get_event(res.event_id).unwrap();
                assert_eq!(ev.status, res.status);
                if res.status == "auto_saved" {
                    assert!(!db.event_classifications(res.event_id).unwrap().is_empty());
                }
            }
            Err(e) => {
                assert!(e.contains("daemon unreachable"), "unexpected error: {e}");
                let db = state.db.lock().unwrap();
                assert_eq!(db.pending_events(10).unwrap().len(), 1);
            }
        }
    }

    #[test]
    fn capture_rejects_empty() {
        let (_p, state) = temp_state();
        let req = CaptureRequest {
            text: "   ".into(),
            date: None,
        };
        assert!(capture(&state, &req).is_err());
        let req2 = CaptureRequest {
            text: "!!!".into(),
            date: None,
        };
        assert!(capture(&state, &req2).is_err());
    }
}