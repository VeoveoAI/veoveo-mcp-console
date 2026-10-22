//! The tools through the SDK's in-process mesh, which exchanges the caller's forwarded
//! token as the Hub does, against fake Hub routes on the mesh. Each fake route records
//! the method, path, query and body it received and the exchanged token's identity the
//! mesh admitted, so a test sees that the console called the Hub as the caller.
//! The fake Hub's answers are synthetic fixture data shaped like the Hub's.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Extension, Router,
    body::Bytes,
    extract::State,
    http::{Method, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::{any, get},
};
use serde_json::{Value, json};
use veoveo_hub_sdk::{
    ExchangedIdentity,
    hosting::{
        Hosted,
        testing::{self, MESH_PROFILE, RunningMesh, TestMesh},
    },
};
use veoveo_mcp_console::{
    SLUG,
    contract::ConsolePermission,
    mcp::{AUTHORIZATION, ConsoleMcp, Hub},
};

/// One request a fake Hub route received.
#[derive(Debug, Clone)]
struct Seen {
    method: Method,
    path: String,
    query: Vec<(String, String)>,
    body: Option<Value>,
    caller: String,
    acting_server: String,
    permissions: Vec<String>,
}

type Requests = Arc<Mutex<Vec<Seen>>>;

/// A repository the fake Hub answers 403 for, as for a caller without a grant.
const DENIED: &str = "repository-without-grant";
/// A repository without an instance.
const MISSING: &str = "repository-without-instance";
const REPOSITORY: &str = "01a1586c-0000-7000-8000-000000000001";
const BRANCH: &str = "dev/jose/x";
const SANDBOX: &str = "8000-000000000002";
const NAMESPACE: &str = "openshell-openshell-vv-factory";

/// The status the fake Hub reports: a branch whose server crashes at start.
fn instance() -> Value {
    json!({
        "deployment": "019a5b30-0000-7000-8000-000000000002",
        "workspace": "vv-factory",
        "sandboxName": SANDBOX,
        "namespace": NAMESPACE,
        "sandbox": {
            "phase": "error",
            "exitCode": 101,
            "restartCount": 3,
            "conditions": [{
                "type": "Ready",
                "status": "False",
                "reason": "MainProcessExited",
                "message": "the main program exited with code 101"
            }]
        },
        "appliedRevision": "a".repeat(64),
        "servingRevision": null,
        "failed": null,
        "warning": "3 instances above the cap of 2"
    })
}

fn problem(status: StatusCode, title: &str, detail: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        json!({
            "type": "https://veoveo.ai/problems/fixture",
            "title": title,
            "status": status.as_u16(),
            "detail": detail
        })
        .to_string(),
    )
        .into_response()
}

/// Every fake Hub route: record the request, then answer as the Hub would.
async fn hub_route(
    State(requests): State<Requests>,
    Extension(identity): Extension<ExchangedIdentity>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    let query: Vec<(String, String)> = uri
        .query()
        .map(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_default();
    let repository = query
        .iter()
        .find(|(name, _)| name == "repository")
        .map(|(_, value)| value.clone());
    requests.lock().unwrap().push(Seen {
        method: method.clone(),
        path: uri.path().to_owned(),
        query,
        body: (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap()),
        caller: identity.caller.actor.id.to_string(),
        acting_server: identity.acting_server.server.to_string(),
        permissions: identity
            .permissions
            .iter()
            .map(ToString::to_string)
            .collect(),
    });
    match (method, uri.path(), repository.as_deref()) {
        (_, _, Some(DENIED)) => problem(
            StatusCode::FORBIDDEN,
            "No grant on the repository",
            "the caller holds no grant on the instance's Lore repository",
        ),
        (Method::GET, _, Some(MISSING)) => problem(
            StatusCode::NOT_FOUND,
            "No instance",
            "no instance serves that branch of the repository",
        ),
        (Method::GET, _, _) => (
            [(header::CACHE_CONTROL, "no-store")],
            axum::Json(instance()),
        )
            .into_response(),
        _ => StatusCode::NO_CONTENT.into_response(),
    }
}

/// The Hub's routes behind the console's tools.
fn hub_routes(requests: Requests) -> Router {
    Router::new()
        .route("/factory/{profile}/instance", get(hub_route))
        .route("/factory/{profile}/watch", any(hub_route))
        .route("/selection/{profile}/{server}", any(hub_route))
        .with_state(requests)
}

/// The console on a mesh whose user holds `granted`, with the fake Hub routes.
async fn console(granted: &[ConsolePermission]) -> (RunningMesh, Requests) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut mesh = TestMesh::bind().await;
    let requests = Requests::default();
    let calls = mesh.register(AUTHORIZATION.clone());
    let hub = Hub::new(mesh.base(), reqwest::Client::new()).unwrap();
    mesh.serve(
        testing::for_domain::<ConsoleMcp>()
            .calls(calls)
            .handler(move || Hosted::new(ConsoleMcp::new(hub.clone())))
            .build(),
    );
    mesh.hub_route(hub_routes(requests.clone()));
    let mesh = mesh
        .granting(granted.iter().map(|permission| permission.name()))
        .start();
    (mesh, requests)
}

async fn call(mesh: &RunningMesh, tool: &str, arguments: Value) -> Result<Value, String> {
    let answer = tokio::time::timeout(Duration::from_secs(30), mesh.call(SLUG, tool, arguments))
        .await
        .expect("the call answers within 30 seconds");
    let result = &answer["result"];
    if result["isError"] == json!(true) {
        return Err(result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned());
    }
    result
        .get("structuredContent")
        .cloned()
        .ok_or_else(|| format!("no result: {answer}"))
}

fn query(seen: &Seen) -> Vec<(&str, &str)> {
    seen.query
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

/// Every request reached the Hub with a token the mesh exchanged for the test user,
/// acting as the console, carrying no permissions.
fn assert_as_the_caller(requests: &[Seen]) {
    assert!(!requests.is_empty());
    for seen in requests {
        assert_eq!(seen.caller, testing::principal().id.to_string(), "{seen:?}");
        assert_eq!(seen.acting_server, SLUG, "{seen:?}");
        assert!(seen.permissions.is_empty(), "{seen:?}");
    }
}

#[tokio::test]
async fn every_tool_requires_the_console_permission_and_nothing_else() {
    let mut names: Vec<String> = AUTHORIZATION
        .requirements
        .tools
        .keys()
        .map(ToString::to_string)
        .collect();
    names.sort();
    assert_eq!(names, ["select", "status", "watch"]);
    for access in AUTHORIZATION.requirements.tools.values() {
        assert_eq!(
            *access,
            veoveo_hub_sdk::registration::RouteAccess::Permission(ConsolePermission::Use.name())
        );
    }
    // Without the console's permission no tool runs and the Hub hears nothing.
    let (denied, requests) = console(&[]).await;
    let answer = denied
        .call(
            SLUG,
            "status",
            json!({"repository": REPOSITORY, "branch": BRANCH}),
        )
        .await;
    assert!(answer.to_string().contains("console.use"), "{answer}");
    assert!(requests.lock().unwrap().is_empty());
}

#[tokio::test]
async fn status_asks_the_hub_as_the_caller_and_waits_at_most_a_minute() {
    let (mesh, requests) = console(&[ConsolePermission::Use]).await;
    let status = call(
        &mesh,
        "status",
        json!({"repository": REPOSITORY, "branch": BRANCH, "wait": 600}),
    )
    .await
    .unwrap();
    assert_eq!(status["sandboxName"], SANDBOX);
    assert_eq!(status["namespace"], NAMESPACE);
    assert_eq!(status["sandbox"]["phase"], "error");
    assert_eq!(status["sandbox"]["exitCode"], 101);
    assert_eq!(status["sandbox"]["restartCount"], 3);
    assert_eq!(
        status["sandbox"]["conditions"][0]["reason"],
        "MainProcessExited"
    );
    assert_eq!(status["appliedRevision"], "a".repeat(64));
    assert_eq!(status["servingRevision"], Value::Null);
    assert_eq!(status["warning"], "3 instances above the cap of 2");

    call(
        &mesh,
        "status",
        json!({"repository": REPOSITORY, "branch": BRANCH}),
    )
    .await
    .unwrap();
    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    for seen in &requests {
        assert_eq!(seen.method, Method::GET);
        assert_eq!(seen.path, format!("/factory/{MESH_PROFILE}/instance"));
        assert_eq!(seen.body, None);
    }
    assert_eq!(
        query(&requests[0]),
        [
            ("repository", REPOSITORY),
            ("branch", BRANCH),
            ("wait", "60")
        ]
    );
    assert_eq!(
        query(&requests[1]),
        [
            ("repository", REPOSITORY),
            ("branch", BRANCH),
            ("wait", "0")
        ]
    );
    assert_as_the_caller(&requests);
}

#[tokio::test]
async fn a_refusal_from_the_hub_is_the_tools_error() {
    let (mesh, requests) = console(&[ConsolePermission::Use]).await;
    let refused = call(
        &mesh,
        "status",
        json!({"repository": DENIED, "branch": BRANCH}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        refused,
        format!(
            "status of `{DENIED}` branch `{BRANCH}`: the Hub answered 403 Forbidden: No grant on \
             the repository: the caller holds no grant on the instance's Lore repository"
        )
    );
    let missing = call(
        &mesh,
        "status",
        json!({"repository": MISSING, "branch": BRANCH}),
    )
    .await
    .unwrap_err();
    assert!(
        missing.contains("404 Not Found: No instance: no instance serves that branch"),
        "{missing}"
    );
    let refused = call(&mesh, "watch", json!({"repository": DENIED}))
        .await
        .unwrap_err();
    assert!(refused.contains("403 Forbidden"), "{refused}");
    assert_as_the_caller(&requests.lock().unwrap());
}

#[tokio::test]
async fn select_sets_and_clears_the_callers_branch_for_a_server() {
    let (mesh, requests) = console(&[ConsolePermission::Use]).await;
    let selected = call(&mesh, "select", json!({"server": "time", "branch": BRANCH}))
        .await
        .unwrap();
    assert_eq!(selected, json!({"server": "time", "branch": BRANCH}));
    let cleared = call(&mesh, "select", json!({"server": "time", "branch": null}))
        .await
        .unwrap();
    assert_eq!(cleared, json!({"server": "time", "branch": null}));

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, Method::PUT);
    assert_eq!(requests[0].path, format!("/selection/{MESH_PROFILE}/time"));
    assert_eq!(requests[0].body, Some(json!({"branch": BRANCH})));
    assert!(requests[0].query.is_empty());
    assert_eq!(requests[1].method, Method::DELETE);
    assert_eq!(requests[1].path, format!("/selection/{MESH_PROFILE}/time"));
    assert_eq!(requests[1].body, None);
    assert_as_the_caller(&requests);
}

#[tokio::test]
async fn watch_asks_the_hub_to_watch_a_repository_as_the_caller() {
    let (mesh, requests) = console(&[ConsolePermission::Use]).await;
    let watching = call(&mesh, "watch", json!({"repository": REPOSITORY}))
        .await
        .unwrap();
    assert_eq!(watching, json!({"repository": REPOSITORY}));
    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, Method::PUT);
    assert_eq!(requests[0].path, format!("/factory/{MESH_PROFILE}/watch"));
    assert_eq!(query(&requests[0]), [("repository", REPOSITORY)]);
    assert_eq!(requests[0].body, None);
    assert_as_the_caller(&requests);
}
