use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub id: i64,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub description: String,
    pub sort: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subdomain {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub sort: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainWithSubs {
    #[serde(flatten)]
    pub domain: Domain,
    pub subdomains: Vec<Subdomain>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewDomain {
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSubdomain {
    pub domain_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Classification {
    pub domain_id: i64,
    pub domain_name: String,
    pub subdomain_id: Option<i64>,
    pub subdomain_name: Option<String>,
    pub probability: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: i64,
    pub raw_text: String,
    pub canonical_topic: String,
    pub event_date: String,
    pub status: String,
    pub confidence: f64,
    pub source: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWithClassifications {
    #[serde(flatten)]
    pub event: Event,
    pub classifications: Vec<Classification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub text: String,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureResult {
    pub event_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewUpdate {
    pub event_id: i64,
    pub domain_ids: Vec<i64>,
    pub subdomain_ids: Vec<Option<i64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DayCount {
    pub date: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsMap(pub Vec<(String, String)>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayaAnswer {
    pub choice: Option<String>,
    pub probabilities: Option<serde_json::Map<String, serde_json::Value>>,
    pub noul: Option<f64>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationOutcome {
    pub status: String,
    pub classifications: Vec<Classification>,
    pub confidence: f64,
    pub reason: String,
}