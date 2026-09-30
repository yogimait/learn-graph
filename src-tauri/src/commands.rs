use tauri::{AppHandle, Emitter, Manager, State};

use crate::models::{
    CaptureRequest, CaptureResult, DayCount, DomainWithSubs, EventWithClassifications, NewDomain,
    NewSubdomain, ReviewUpdate, SettingsMap,
};
use crate::pipeline::{self, AppState, EVENT_CAPTURED, EVENT_CLASSIFIED};

#[tauri::command]
pub fn capture_event(
    state: State<'_, AppState>,
    app: AppHandle,
    req: CaptureRequest,
) -> Result<CaptureResult, String> {
    eprintln!("capture_event: {:?}", req.text);
    let result = pipeline::capture(&state, &req)?;
    let _ = app.emit(EVENT_CAPTURED, &result);
    let _ = app.emit(EVENT_CLASSIFIED, result.event_id);
    Ok(result)
}

#[tauri::command]
pub fn review_event(
    state: State<'_, AppState>,
    app: AppHandle,
    update: ReviewUpdate,
) -> Result<(), String> {
    pipeline::apply_review(&state, &app, &update)
}

#[tauri::command]
pub fn list_domains(state: State<'_, AppState>) -> Result<Vec<DomainWithSubs>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_domains().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_domain(state: State<'_, AppState>, domain: NewDomain) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_domain(&domain).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_domain(
    state: State<'_, AppState>,
    id: i64,
    name: String,
    description: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.update_domain(id, &name, &description).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_domain(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_domain(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_subdomain(state: State<'_, AppState>, sub: NewSubdomain) -> Result<i64, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.add_subdomain(&sub).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_subdomain(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.delete_subdomain(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_events(
    state: State<'_, AppState>,
    status: Option<String>,
    limit: i64,
) -> Result<Vec<EventWithClassifications>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.list_events(status.as_deref(), limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn daily_counts(
    state: State<'_, AppState>,
    domain_id: Option<i64>,
    from: String,
    to: String,
) -> Result<Vec<DayCount>, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.daily_counts(domain_id, &from, &to).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsMap, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = db
        .0
        .prepare("SELECT key, value FROM settings ORDER BY key")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(SettingsMap(rows))
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.set_setting(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn daemon_health(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    crate::laya::daemon_health(&db)
}

#[tauri::command]
pub fn send_to_review(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    db.send_to_review(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn diag(msg: String) {
    eprintln!("diag: {msg}");
}

#[tauri::command]
pub fn hide_capture_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("capture") {
        let _ = win.hide();
    }
}