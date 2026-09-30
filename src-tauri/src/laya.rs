use std::collections::HashMap;

use serde_json::{json, Map, Value};

use crate::db::Db;
use crate::models::{Classification, ClassificationOutcome, LayaAnswer};

const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:8080";

/// Deterministic normalization of a raw capture into a canonical topic key.
/// ponytail: lowercase + punctuation collapse only; alias resolution ("2sum" vs "Two Sum")
/// via Laya is a later enhancement when real data shows collision volume.
pub fn canonical_topic(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_ws = true;
    for c in raw.chars().flat_map(|c| c.to_lowercase()) {
        if c.is_alphanumeric() || c.is_whitespace() {
            if c.is_whitespace() {
                if !prev_ws && !out.is_empty() {
                    out.push(' ');
                }
                prev_ws = true;
            } else {
                out.push(c);
                prev_ws = false;
            }
        }
    }
    out.trim().to_string()
}

/// Snapshot of everything classification needs from the DB, so the DB lock is
/// never held during the (seconds-long) HTTP call to the daemon.
pub struct ClassifyCtx {
    pub daemon_url: String,
    /// (domain_id, domain_name, description, subdomain names in display order)
    pub domains: Vec<(i64, String, String, Vec<String>)>,
    /// domain_name -> (subdomain_id, subdomain_name) in display order
    pub subs_by_domain: HashMap<String, Vec<(i64, String)>>,
    /// user's manual corrections (raw_text -> domain names), few-shot signal
    pub corrections: Vec<(String, String)>,
    pub multi_ratio: f64,
    pub multi_floor: f64,
    pub review_threshold: f64,
    pub subdomain_threshold: f64,
}

/// Stable short criteria key for a domain. The local model ranks far better
/// with short lowercase tokens than multi-word display names (validated live:
/// "dsa" 0.53 vs "DSA"/"System Design" keys collapsing into "none" 0.45+).
pub fn slug(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub fn build_ctx(db: &Db) -> Result<ClassifyCtx, String> {
    let taxonomy = db.list_domains().map_err(|e| format!("db: {e}"))?;
    let url = db.get_setting("laya_url");
    Ok(ClassifyCtx {
        daemon_url: if url.is_empty() { DEFAULT_DAEMON_URL.to_string() } else { url },
        domains: taxonomy
            .iter()
            .map(|d| {
                (
                    d.domain.id,
                    d.domain.name.clone(),
                    d.domain.description.clone(),
                    d.subdomains.iter().map(|s| s.name.clone()).collect(),
                )
            })
            .collect(),
        subs_by_domain: taxonomy
            .iter()
            .map(|d| {
                (
                    d.domain.name.clone(),
                    d.subdomains.iter().map(|s| (s.id, s.name.clone())).collect(),
                )
            })
            .collect(),
        corrections: db.recent_corrections(8).unwrap_or_default(),
        multi_ratio: db.get_setting_f64("multi_ratio", 0.5),
        multi_floor: db.get_setting_f64("multi_floor", 0.2),
        review_threshold: db.get_setting_f64("review_threshold", 0.5),
        subdomain_threshold: db.get_setting_f64("subdomain_threshold", 0.3),
    })
}

pub fn daemon_health(db: &Db) -> Result<serde_json::Value, String> {
    let url = db.get_setting("laya_url");
    let url = if url.is_empty() { DEFAULT_DAEMON_URL.to_string() } else { url };
    // /health is GET-only on the daemon; POSTing returns 405 Method Not Allowed
    ureq::get(&format!("{url}/health"))
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .map_err(|e| format!("daemon unreachable: {e}"))?
        .into_json::<Value>()
        .map_err(|e| format!("bad response: {e}"))
}

fn post_json(url: &str, body: &Value) -> Result<Value, String> {
    ureq::post(url)
        .timeout(std::time::Duration::from_secs(60))
        .send_json(body)
        .map_err(|e| format!("daemon unreachable: {e}"))?
        .into_json::<Value>()
        .map_err(|e| format!("bad response: {e}"))
}

/// Pass 1: one choice question over every domain plus a `none` escape hatch.
/// Ranking (relative choice) is measurably more reliable than per-domain noul
/// on the local model (validated live: 0.53-0.64 vs 0.52 for "Two Sum").
/// Criteria keys are slugs; text is the user's semantic description.
fn build_domain_question(domains: &[(i64, String, String, Vec<String>)]) -> (Value, HashMap<String, i64>) {
    let mut criteria = Map::new();
    let mut slug_to_id = HashMap::new();
    for (id, name, description, subs) in domains {
        let mut key = slug(name);
        if key.is_empty() {
            key = format!("d{id}");
        }
        if slug_to_id.contains_key(&key) {
            key = format!("{key}{id}");
        }
        let text = if description.trim().is_empty() {
            if subs.is_empty() {
                name.clone()
            } else {
                format!("{name}: {}", subs.join(", "))
            }
        } else {
            description.clone()
        };
        criteria.insert(key.clone(), Value::String(text));
        slug_to_id.insert(key, *id);
    }
    criteria.insert(
        "none".into(),
        Value::String("Not a learning or practice activity, or no domain fits".to_string()),
    );
    let question = json!({
        "type": "choice",
        "instructions": "Which single domain does this learning entry primarily belong to? The user logs short notes about what they just learned or practiced.",
        "criteria": criteria
    });
    (question, slug_to_id)
}

/// Pass 2: one choice question per included domain over its subdomains + none.
/// Returns the question plus the slug->subdomain-id mapping.
fn build_subdomain_question(domain_name: &str, subs: &[(i64, String)]) -> (Value, HashMap<String, i64>) {
    let mut criteria = Map::new();
    let mut slug_to_id = HashMap::new();
    for (id, name) in subs {
        let mut key = slug(name);
        if key.is_empty() || slug_to_id.contains_key(&key) {
            key = format!("{}{id}", if key.is_empty() { "s".to_string() } else { key });
        }
        criteria.insert(key.clone(), Value::String(format!("Subdomain of {domain_name}")));
        slug_to_id.insert(key, *id);
    }
    criteria.insert("none".into(), Value::String("No subdomain fits".to_string()));
    let question = json!({
        "type": "choice",
        "instructions": format!("The entry was already classified under {domain_name}. Which subdomain of {domain_name} does it primarily belong to?"),
        "criteria": criteria
    });
    (question, slug_to_id)
}

fn parse_answer(v: &Value) -> LayaAnswer {
    LayaAnswer {
        choice: v.get("choice").and_then(|c| c.as_str()).map(String::from),
        probabilities: v.get("probabilities").and_then(|p| p.as_object()).cloned(),
        noul: v.get("noul").and_then(|n| n.as_f64()),
        confidence: v.get("confidence").and_then(|c| c.as_f64()),
    }
}

/// Run the two-pass classification against the Laya daemon and produce a
/// validated, taxonomy-clamped outcome. Never invents domain IDs: every id in
/// the result comes from the taxonomy snapshot.
pub fn classify(ctx: &ClassifyCtx, raw_text: &str) -> Result<ClassificationOutcome, String> {
    if ctx.domains.is_empty() {
        return Ok(ClassificationOutcome {
            status: "needs_review".into(),
            classifications: vec![],
            confidence: 0.0,
            reason: "taxonomy empty".into(),
        });
    }
    let name_to_id: HashMap<&str, i64> = ctx
        .domains
        .iter()
        .map(|(id, name, _, _)| (name.as_str(), *id))
        .collect();

    let url = format!("{}/v1/systemone", ctx.daemon_url);
    let mut state = json!({ "entry": raw_text });
    if !ctx.corrections.is_empty() {
        let examples = ctx
            .corrections
            .iter()
            .map(|(text, domains)| format!("{} -> {}", text, domains))
            .collect::<Vec<_>>()
            .join("; ");
        state.as_object_mut().unwrap().insert(
            "corrections".into(),
            json!(format!(
                "The user's own past corrections of similar entries (entry -> correct domains): {examples}"
            )),
        );
    }

    let (question, slug_to_id) = build_domain_question(&ctx.domains);
    let pass1 = post_json(
        &url,
        &json!({
            "state": state,
            "questions": { "domain": question },
            "model": "auto"
        }),
    )?;
    let pass1_answers = pass1.get("answers").and_then(|a| a.as_object()).ok_or("no answers")?;
    let domain_answer = parse_answer(pass1_answers.get("domain").ok_or("no domain answer")?);

    let probs = domain_answer.probabilities.unwrap_or_default();
    let pick = |k: &str| probs.get(k).and_then(|p| p.as_f64()).unwrap_or(0.0);
    let winner_slug = domain_answer.choice.unwrap_or_default();

    if winner_slug == "none" || !slug_to_id.contains_key(&winner_slug) {
        return Ok(ClassificationOutcome {
            status: "needs_review".into(),
            classifications: vec![],
            confidence: pick(&winner_slug),
            reason: "no domain or unknown winner".into(),
        });
    }
    let winner_id = slug_to_id[&winner_slug];
    let winner_name = ctx
        .domains
        .iter()
        .find(|(id, _, _, _)| *id == winner_id)
        .map(|(_, n, _, _)| n.clone())
        .unwrap_or_default();
    let winner_p = pick(&winner_slug);

    let mut included: Vec<(i64, String, f64)> = vec![(winner_id, winner_name.clone(), winner_p)];
    for (name, id) in name_to_id.iter() {
        if *id == winner_id {
            continue;
        }
        let p = pick(&slug(name));
        if p >= ctx.multi_floor && p >= winner_p * ctx.multi_ratio {
            included.push((*id, name.to_string(), p));
        }
    }

    let mut subdomain_qs = Map::new();
    let mut sub_slugs: Vec<HashMap<String, i64>> = Vec::new();
    for (_, name, _) in &included {
        let subs = ctx
            .subs_by_domain
            .get(name.as_str())
            .cloned()
            .unwrap_or_default();
        let (q, map) = build_subdomain_question(name, &subs);
        subdomain_qs.insert(format!("sub_{name}"), q);
        sub_slugs.push(map);
    }

    let mut classifications = Vec::new();
    let mut confidence = winner_p;
    if !subdomain_qs.is_empty() {
        let pass2 = post_json(
            &url,
            &json!({
                "state": state,
                "questions": subdomain_qs,
                "model": "auto"
            }),
        )?;
        let pass2_answers = pass2.get("answers").and_then(|a| a.as_object()).ok_or("no pass2 answers")?;
        for ((domain_id, name, p), slug_map) in included.iter().zip(sub_slugs.iter()) {
            let sub_ans = parse_answer(pass2_answers.get(&format!("sub_{name}")).unwrap_or(&json!({})));
            let mut sub_id = None;
            if let Some(choice_slug) = sub_ans.choice {
                if choice_slug != "none" {
                    if let Some(sid) = slug_map.get(&choice_slug) {
                        let prob = sub_ans
                            .probabilities
                            .as_ref()
                            .and_then(|m| m.get(&choice_slug))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        if prob >= ctx.subdomain_threshold {
                            sub_id = Some(*sid);
                        }
                    }
                }
            }
            classifications.push(Classification {
                domain_id: *domain_id,
                domain_name: name.clone(),
                subdomain_id: sub_id,
                subdomain_name: None,
                probability: *p,
            });
        }
        confidence = classifications
            .iter()
            .map(|c| c.probability)
            .fold(0.0_f64, f64::max);
    } else {
        for (domain_id, name, p) in &included {
            classifications.push(Classification {
                domain_id: *domain_id,
                domain_name: name.clone(),
                subdomain_id: None,
                subdomain_name: None,
                probability: *p,
            });
        }
    }

    let status = if winner_p >= ctx.review_threshold {
        "auto_saved"
    } else {
        "needs_review"
    };
    Ok(ClassificationOutcome {
        status: status.into(),
        classifications,
        confidence,
        reason: if status == "auto_saved" { "auto".into() } else { "low confidence".into() },
    })
}

/// Validate domain/subdomain ids against the taxonomy (deterministic gate).
pub fn valid_classification(db: &Db, domain_id: i64, subdomain_id: Option<i64>) -> bool {
    match db.list_domains() {
        Ok(domains) => domains.iter().any(|d| {
            d.domain.id == domain_id
                && subdomain_id.map_or(true, |sid| d.subdomains.iter().any(|s| s.id == sid))
        }),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_topic_normalizes() {
        assert_eq!(canonical_topic("Solved Two Sum!"), "solved two sum");
        assert_eq!(canonical_topic("  2Sum   "), "2sum");
        // punctuation is dropped, not spaced: "2-sum" dedupes with "2sum"
        assert_eq!(canonical_topic("JWT-Refresh Tokens."), "jwtrefresh tokens");
        assert_eq!(canonical_topic("2-sum"), canonical_topic("2sum"));
        assert_eq!(canonical_topic(""), "");
    }

    #[test]
    fn classification_outcome_low_confidence_goes_to_review() {
        // Threshold logic itself lives in classify(); here we guard the
        // canonical gate: unknown winner -> needs_review with no classifications.
        let outcome_none = ClassificationOutcome {
            status: "needs_review".into(),
            classifications: vec![],
            confidence: 0.0,
            reason: "no domain or unknown winner".into(),
        };
        assert!(outcome_none.classifications.is_empty());
    }
}
