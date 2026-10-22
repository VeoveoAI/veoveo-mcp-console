//! The Hub's own routes, called for the tool's caller with the bearer the SDK exchanges
//! for the caller's profile. The Hub decides each call; its refusals reach the agent
//! with their status and RFC 9457 problem detail.
use std::{fmt, time::Duration};

use reqwest::{
    Method, StatusCode,
    header::{ACCEPT, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use veoveo_hub_sdk::{HubProfileId, client::CallError, client::HubClient};

use crate::contract::InstanceStatus;

/// The longest `status` waits for an instance to settle, as the Hub bounds it.
pub const MAX_WAIT: Duration = Duration::from_secs(60);
/// How long a request may take beyond the wait it asks for.
const REQUEST_MARGIN: Duration = Duration::from_secs(15);

/// The Hub at the origin of its issuer.
#[derive(Clone)]
pub struct Hub {
    base: url::Url,
    http: reqwest::Client,
}

/// Why a call to the Hub produced no answer.
#[derive(Debug, thiserror::Error)]
pub enum HubError {
    /// The SDK obtained no bearer for the caller.
    #[error("no token for the caller: {0}")]
    Bearer(#[from] CallError),
    #[error("the Hub could not be reached: {0}")]
    Transport(#[from] reqwest::Error),
    /// The Hub answered with an error status.
    #[error("the Hub answered {status}{problem}")]
    Answered {
        status: StatusCode,
        problem: Problem,
    },
    #[error("the Hub's answer is not what console reads: {0}")]
    Decode(#[from] serde_json::Error),
}

/// The RFC 9457 problem detail the Hub answers an error with.
#[derive(Debug, Default, Deserialize)]
pub struct Problem {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in [&self.title, &self.detail].into_iter().flatten() {
            write!(f, ": {part}")?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
struct SelectionBody<'a> {
    branch: &'a str,
}

impl Hub {
    /// The Hub whose issuer is `issuer`; its routes are at the issuer's origin.
    pub fn new(issuer: &url::Url, http: reqwest::Client) -> anyhow::Result<Self> {
        anyhow::ensure!(
            matches!(issuer.scheme(), "http" | "https") && issuer.has_host(),
            "the Hub issuer is no HTTP origin"
        );
        Ok(Self {
            base: issuer.clone(),
            http,
        })
    }

    /// `GET /factory/{profile}/instance?repository=&branch=&wait=`: the instance of
    /// `branch` of `repository`, after waiting up to `wait`, at most [`MAX_WAIT`], for
    /// its applied revision to serve or fail.
    pub async fn status(
        &self,
        client: &HubClient,
        profile: &HubProfileId,
        repository: &str,
        branch: &str,
        wait: Duration,
    ) -> Result<InstanceStatus, HubError> {
        let wait = wait.min(MAX_WAIT);
        let url = self.route(
            &["factory", profile.as_str(), "instance"],
            &[
                ("repository", repository),
                ("branch", branch),
                ("wait", &wait.as_secs().to_string()),
            ],
        );
        let answer = self
            .send(client, Method::GET, url, None, wait + REQUEST_MARGIN)
            .await?;
        decode(&answer)
    }

    /// `PUT /selection/{profile}/{server}` with `{"branch": …}`, or `DELETE` of the same
    /// path for the base branch.
    pub async fn select(
        &self,
        client: &HubClient,
        profile: &HubProfileId,
        server: &str,
        branch: Option<&str>,
    ) -> Result<(), HubError> {
        let url = self.route(&["selection", profile.as_str(), server], &[]);
        let (method, body) = match branch {
            Some(branch) => (
                Method::PUT,
                Some(serde_json::to_vec(&SelectionBody { branch })?),
            ),
            None => (Method::DELETE, None),
        };
        self.send(client, method, url, body, REQUEST_MARGIN)
            .await
            .map(drop)
    }

    /// `PUT /factory/{profile}/watch?repository=`: the Hub watches `repository` as the
    /// caller.
    pub async fn watch(
        &self,
        client: &HubClient,
        profile: &HubProfileId,
        repository: &str,
    ) -> Result<(), HubError> {
        let url = self.route(
            &["factory", profile.as_str(), "watch"],
            &[("repository", repository)],
        );
        self.send(client, Method::PUT, url, None, REQUEST_MARGIN)
            .await
            .map(drop)
    }

    /// The Hub route of `segments`, each percent-encoded as one path segment.
    fn route(&self, segments: &[&str], query: &[(&str, &str)]) -> url::Url {
        let mut url = self.base.clone();
        url.set_query(None);
        url.set_fragment(None);
        url.path_segments_mut()
            .expect("an HTTP origin has a path")
            .clear()
            .extend(segments);
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        url
    }

    async fn send(
        &self,
        client: &HubClient,
        method: Method,
        url: url::Url,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<Vec<u8>, HubError> {
        let bearer = client.hub_bearer().await?;
        let mut request = self
            .http
            .request(method, url)
            .bearer_auth(bearer.expose())
            .header(ACCEPT, "application/json")
            .timeout(timeout);
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").body(body);
        }
        let response = request.send().await?;
        let status = response.status();
        let body = response.bytes().await?.to_vec();
        if status.is_success() {
            return Ok(body);
        }
        Err(HubError::Answered {
            status,
            problem: serde_json::from_slice(&body).unwrap_or_default(),
        })
    }
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, HubError> {
    Ok(serde_json::from_slice(body)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_encode_each_segment_at_the_issuers_origin() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let hub = Hub::new(
            &url::Url::parse("http://vv-hub.internal:8080/ignored?x=1").unwrap(),
            reqwest::Client::new(),
        )
        .unwrap();
        let route = hub.route(
            &["selection", "default", "../admin"],
            &[("branch", "dev/a b")],
        );
        assert_eq!(
            route.as_str(),
            "http://vv-hub.internal:8080/selection/default/..%2Fadmin?branch=dev%2Fa+b"
        );
        assert!(Hub::new(&url::Url::parse("urn:hub").unwrap(), reqwest::Client::new()).is_err());
    }

    #[test]
    fn a_refusal_names_the_hubs_problem() {
        let error = HubError::Answered {
            status: StatusCode::FORBIDDEN,
            problem: serde_json::from_str(
                r#"{"type":"about:blank","title":"No grant on the repository","detail":"the caller holds no grant on the instance's Lore repository","status":403}"#,
            )
            .unwrap(),
        };
        assert_eq!(
            error.to_string(),
            "the Hub answered 403 Forbidden: No grant on the repository: the caller holds no grant on the instance's Lore repository"
        );
    }
}
