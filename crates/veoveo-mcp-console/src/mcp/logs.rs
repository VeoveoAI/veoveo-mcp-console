//! The installation's log store, VictoriaLogs, which Vector fills with every
//! container's output and its pod's labels as `kubernetes.pod_labels.*` fields. A
//! sandbox's workload pod and its supervisor pod both carry the sandbox's OpenShell ID
//! as `openshell.ai/boundary-pair`, so one query by that label returns the main
//! program's output, from container `agent`, and OpenShell's network denials for the
//! sandbox, from the supervisor. The `status` call before the query is the access
//! check; the log store needs no credentials.
use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::{StatusCode, header::ACCEPT};
use serde::Deserialize;

use crate::contract::{LogLine, LogSource, MAX_LOG_LINE_BYTES};

/// The container of a sandbox's main program.
pub const WORKLOAD_CONTAINER: &str = "agent";
/// Vector's field for a pod's `openshell.ai/boundary-pair` label.
const PAIR_FIELD: &str = "kubernetes.pod_labels.openshell.ai/boundary-pair";
/// Vector's field for a pod's `openshell.ai/boundary-role` label.
const ROLE_FIELD: &str = "kubernetes.pod_labels.openshell.ai/boundary-role";
/// The role of the pod that writes OpenShell's events for a sandbox.
const SUPERVISOR: &str = "supervisor";
/// How long one query may take.
const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
/// The most bytes of a refusal's body an error carries.
const MAX_ERROR_BYTES: usize = 512;

/// VictoriaLogs' LogsQL query endpoint, `{base}/select/logsql/query`.
#[derive(Clone)]
pub struct LogStore {
    query: url::Url,
    http: reqwest::Client,
}

/// Why the log store produced no lines.
#[derive(Debug, thiserror::Error)]
pub enum LogStoreError {
    #[error("the log store could not be reached: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("the log store answered {status}: {body}")]
    Answered { status: StatusCode, body: String },
    #[error("a log record is not what console reads: {0}")]
    Decode(#[from] serde_json::Error),
}

/// The latest lines of a sandbox, oldest first, and whether older ones were left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub lines: Vec<LogLine>,
    pub truncated: bool,
}

/// One record as the query's `fields` pipe returns it.
#[derive(Deserialize)]
struct Record {
    #[serde(rename = "_time")]
    time: DateTime<Utc>,
    #[serde(rename = "_msg", default)]
    message: String,
    #[serde(default)]
    stream: String,
    #[serde(rename = "kubernetes.pod_labels.openshell.ai/boundary-role", default)]
    role: String,
}

impl LogStore {
    /// The log store at `base`, such as
    /// `http://victorialogs.veoveo-system.svc.cluster.local:9428`.
    pub fn new(base: &url::Url, http: reqwest::Client) -> anyhow::Result<Self> {
        anyhow::ensure!(
            matches!(base.scheme(), "http" | "https") && base.has_host(),
            "the log store URL is no HTTP origin"
        );
        let mut query = base.clone();
        query.set_query(None);
        query.set_fragment(None);
        query
            .path_segments_mut()
            .expect("an HTTP origin has a path")
            .pop_if_empty()
            .extend(["select", "logsql", "query"]);
        Ok(Self { query, http })
    }

    /// The latest `limit` lines of the sandbox `sandbox_id` from `since` on: its main
    /// program's output, marked by stream, and OpenShell's network denials.
    pub async fn read(
        &self,
        sandbox_id: &str,
        since: Option<DateTime<Utc>>,
        limit: usize,
    ) -> Result<Lines, LogStoreError> {
        // One record past the limit tells whether older lines exist.
        let records = self.query(&query(sandbox_id, limit + 1), since).await?;
        let truncated = records.len() > limit;
        let mut lines: Vec<LogLine> = records.into_iter().take(limit).map(line).collect();
        lines.sort_by_key(|line| line.time);
        Ok(Lines { lines, truncated })
    }

    async fn query(
        &self,
        query: &str,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<Record>, LogStoreError> {
        let mut url = self.query.clone();
        {
            let mut parameters = url.query_pairs_mut();
            parameters.append_pair("query", query);
            if let Some(since) = since {
                parameters
                    .append_pair("start", &since.to_rfc3339_opts(SecondsFormat::AutoSi, true));
            }
        }
        let response = self
            .http
            .get(url)
            .header(ACCEPT, "application/json")
            .timeout(QUERY_TIMEOUT)
            .send()
            .await?;
        let status = response.status();
        let body = response.bytes().await?;
        if !status.is_success() {
            return Err(LogStoreError::Answered {
                status,
                body: bounded(String::from_utf8_lossy(&body).trim(), MAX_ERROR_BYTES),
            });
        }
        // One JSON object per line, newest first.
        body.split(|byte| *byte == b'\n')
            .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
            .map(|line| serde_json::from_slice(line).map_err(LogStoreError::from))
            .collect()
    }
}

/// The latest `limit` records of the sandbox, newest first: its main program's output
/// and its supervisor's OCSF events whose action is `DENIED`.
fn query(sandbox_id: &str, limit: usize) -> String {
    format!(
        "{}:={} ({}:={} or ({}:={} OCSF DENIED)) \
         | sort by (_time desc) limit {limit} | fields _time, _msg, stream, {}",
        quoted(PAIR_FIELD),
        quoted(sandbox_id),
        "kubernetes.container_name",
        quoted(WORKLOAD_CONTAINER),
        quoted(ROLE_FIELD),
        quoted(SUPERVISOR),
        quoted(ROLE_FIELD),
    )
}

/// A LogsQL quoted string.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).expect("a string serializes")
}

fn line(record: Record) -> LogLine {
    let source = if record.role == SUPERVISOR {
        LogSource::Denied
    } else if record.stream == "stderr" {
        LogSource::Stderr
    } else {
        LogSource::Stdout
    };
    LogLine {
        time: record.time,
        source,
        text: bounded(&record.message, MAX_LOG_LINE_BYTES),
    }
}

/// `text` within `max` bytes, cut at a character boundary and ending in `…` when cut.
fn bounded(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_owned();
    }
    let mut end = max - '…'.len_utf8();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &text[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_query_selects_the_output_and_the_supervisors_denials() {
        // The query that returned exactly the sandbox's output and denials from
        // VictoriaLogs v1.53.0 in the probe recorded in the crate's DESIGN.md.
        assert_eq!(
            query("fc754186-814d-4aa2-bb05-13378da8acc9", 201),
            "\"kubernetes.pod_labels.openshell.ai/boundary-pair\":=\"fc754186-814d-4aa2-bb05-13378da8acc9\" \
             (kubernetes.container_name:=\"agent\" or \
             (\"kubernetes.pod_labels.openshell.ai/boundary-role\":=\"supervisor\" OCSF DENIED)) \
             | sort by (_time desc) limit 201 \
             | fields _time, _msg, stream, \"kubernetes.pod_labels.openshell.ai/boundary-role\""
        );
        // A value cannot close its quotes.
        assert_eq!(quoted("a\" or *"), "\"a\\\" or *\"");
    }

    #[test]
    fn long_lines_are_cut_at_a_character_boundary() {
        assert_eq!(bounded("short", 8), "short");
        let cut = bounded(&"é".repeat(10), 8);
        assert!(cut.len() <= 8, "{cut}");
        assert_eq!(cut, "éé…");
    }

    #[test]
    fn the_query_endpoint_is_under_the_base() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let store = LogStore::new(
            &url::Url::parse("http://victorialogs.veoveo-system.svc.cluster.local:9428").unwrap(),
            reqwest::Client::new(),
        )
        .unwrap();
        assert_eq!(
            store.query.as_str(),
            "http://victorialogs.veoveo-system.svc.cluster.local:9428/select/logsql/query"
        );
    }
}
