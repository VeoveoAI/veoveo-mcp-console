//! The tools through the SDK's in-process mesh, which exchanges the caller's forwarded
//! token as the Hub does, against fake Hub routes on the mesh and a fake log store. Each
//! fake Hub route records the method, path, query and body it received and the
//! exchanged token's identity the mesh admitted, so a test sees that the console called
//! the Hub as the caller; the fake log store records each query. The answers are
//! synthetic fixture data shaped like the Hub's and like VictoriaLogs v1.53.0's.
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
    mcp::{AUTHORIZATION, ConsoleMcp, Hub, LogStore},
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
/// A repository whose instance has no sandbox at OpenShell.
const NO_SANDBOX: &str = "repository-without-sandbox";
/// A repository whose sandbox ID the fake log store refuses to query.
const BROKEN_STORE: &str = "repository-with-unreadable-logs";
const UNREADABLE_SANDBOX_ID: &str = "unreadable";
const REPOSITORY: &str = "01a1586c-0000-7000-8000-000000000001";
const BRANCH: &str = "dev/jose/x";
const SANDBOX: &str = "8000-000000000002";
const SANDBOX_ID: &str = "fc754186-814d-4aa2-bb05-13378da8acc9";
/// A repository name the fake Hub answers 409 for, as one Lore holds already.
const TAKEN: &str = "taken";
/// A repository name whose creation the fake Hub answers 503 for, created in Lore but
/// not registered, unless the call names the repository's `id`.
const UNREGISTERED: &str = "left-unregistered";
/// The ID of every repository the fake Hub creates.
const CREATED: &str = "5f0c1d2e3a4b5c6d7e8f90a1b2c3d4e5";
const LORE_REMOTE: &str = "https://lore.veoveo.example";

/// The version the fake Hub's Computer runs, and the current one.
const RUNNING: &str = "3f2a9c0d1e4b";
const CURRENT: &str = "8b1c2d3e4f5a";

/// The fake Hub's answer for the caller's Computer: an update is available, or with
/// `restarting`, the restart to it runs.
fn computer_status(restarting: bool) -> Value {
    let mut status = json!({
        "computer": "019a5b30-0000-7000-8000-0000000000c1",
        "version": {"version": RUNNING},
        "sandbox": "ready",
        "current": {"version": CURRENT, "summary": "Faster builds"},
        "updateAvailable": true,
        "whatsNew": {
            "note": "Faster builds\nThe image adds a compiler.",
            "image": {"from": "registry.internal/computer@sha256:1", "to": "registry.internal/computer@sha256:2"},
            "networkRulesAdded": [{"name": "model", "endpoints": ["model.example:443"]}]
        }
    });
    if restarting {
        status["restart"] = json!({
            "version": {"version": CURRENT, "summary": "Faster builds"},
            "since": "2026-10-23T12:00:00Z"
        });
    }
    status
}

/// The status the fake Hub reports: a branch whose server crashes at start.
fn instance() -> Value {
    json!({
        "deployment": "019a5b30-0000-7000-8000-000000000002",
        "workspace": "vv-factory",
        "sandboxName": SANDBOX,
        "sandboxId": SANDBOX_ID,
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
    problem_with(status, title, detail, json!({}))
}

/// A problem with the extension members of `extensions`.
fn problem_with(status: StatusCode, title: &str, detail: &str, extensions: Value) -> Response {
    let mut problem = json!({
        "type": "https://veoveo.ai/problems/fixture",
        "title": title,
        "status": status.as_u16(),
        "detail": detail
    });
    problem
        .as_object_mut()
        .unwrap()
        .extend(extensions.as_object().unwrap().clone());
    (
        status,
        [(header::CONTENT_TYPE, "application/problem+json")],
        problem.to_string(),
    )
        .into_response()
}

/// The Hub's answer to `POST /lore/{profile}/repositories` with `body`.
fn created(body: &Value) -> Response {
    let name = body["name"].as_str().unwrap_or_default();
    let id = body["id"].as_str();
    match (name, id) {
        (TAKEN, _) => problem(
            StatusCode::CONFLICT,
            "Lore holds a repository of this ID or name already",
            "a repository of this ID has another name, or this name belongs to another ID",
        ),
        (UNREGISTERED, None) => problem_with(
            StatusCode::SERVICE_UNAVAILABLE,
            "Repository creation is unavailable",
            "The repository was created in Lore but not registered. Call again with this id to register it.",
            json!({"id": CREATED}),
        ),
        (name, id) => {
            let id = id.unwrap_or(CREATED);
            (
                StatusCode::CREATED,
                [(header::CACHE_CONTROL, "no-store")],
                axum::Json(
                    json!({"id": id, "name": name, "remote": format!("{LORE_REMOTE}/{id}")}),
                ),
            )
                .into_response()
        }
    }
}

/// An email the fake Hub knows no person by, as for someone who never signed in.
const STRANGER: &str = "stranger@example.com";
/// A person whose removal the fake Hub refuses, as the caller's own last owner role.
const SELF: &str = "me@example.com";

/// The fake Hub's answer for who may reach `repository`.
fn access(repository: &str) -> Value {
    json!({
        "repository": repository,
        "access": [
            {"person": {"email": SELF, "name": "Me"}, "access": "owner", "actions": ["owner"]},
            {"person": {"email": null, "name": null}, "access": "member", "actions": []},
            {"group": {"name": "flight"}, "access": "member", "actions": []}
        ]
    })
}

/// The fake Hub's group `name`, with the caller as its owner.
/// The ID the Hub answers for every group of the fixture.
const GROUP_ID: &str = "5f0c1d2e3a4b5c6d7e8f90a1b2c3d4e5";

fn group(name: &str) -> Value {
    json!({"id": GROUP_ID, "name": name, "members": [
        {"email": SELF, "name": "Me", "role": "owner"},
        {"email": "ana@example.com", "name": "Ana", "role": "member"}
    ]})
}

/// The Hub's answer to the access, group and membership routes.
fn sharing(method: &Method, path: &str, body: &Value) -> Option<Response> {
    let segments: Vec<&str> = path.trim_start_matches('/').split('/').collect();
    let answer = match (method, segments.as_slice()) {
        (&Method::GET, ["lore", _, "repositories", repository, "access"]) => {
            axum::Json(access(repository)).into_response()
        }
        (&Method::PUT, ["lore", _, "repositories", repository, "access"]) => {
            if body["person"] == json!(STRANGER) {
                problem(
                    StatusCode::NOT_FOUND,
                    "Unknown person",
                    "nobody with this email has signed in yet; they can be added once they sign in",
                )
            } else {
                axum::Json(access(repository)).into_response()
            }
        }
        (&Method::GET, ["lore", _, "groups"]) => {
            axum::Json(json!({"groups": [group("flight")]})).into_response()
        }
        (&Method::POST, ["lore", _, "groups"]) => (
            StatusCode::CREATED,
            axum::Json(json!({"id": GROUP_ID, "name": body["name"], "members": [
                {"email": SELF, "name": "Me", "role": "owner"}
            ]})),
        )
            .into_response(),
        (&Method::PUT, ["lore", _, "groups", name, "members"]) => {
            if body["person"] == json!(SELF) && body["role"] != json!("owner") {
                problem(
                    StatusCode::CONFLICT,
                    "The group would have no owner",
                    "you would no longer own the group; make another member its owner first",
                )
            } else {
                axum::Json(group(name)).into_response()
            }
        }
        _ => return None,
    };
    Some(answer)
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
    let body: Option<Value> = (!body.is_empty()).then(|| serde_json::from_slice(&body).unwrap());
    requests.lock().unwrap().push(Seen {
        method: method.clone(),
        path: uri.path().to_owned(),
        query,
        body: body.clone(),
        caller: identity.caller.actor.id.to_string(),
        acting_server: identity.acting_server.server.to_string(),
        permissions: identity
            .permissions
            .iter()
            .map(ToString::to_string)
            .collect(),
    });
    if let Some(answer) = sharing(&method, uri.path(), &body.clone().unwrap_or_default()) {
        return answer;
    }
    match (method, uri.path(), repository.as_deref()) {
        (Method::POST, path, _) if path.ends_with("/repositories") => {
            created(&body.unwrap_or_default())
        }
        (Method::GET, path, _) if path.starts_with("/computer/") => (
            [(header::CACHE_CONTROL, "no-store")],
            axum::Json(computer_status(false)),
        )
            .into_response(),
        (Method::POST, path, _) if path.starts_with("/computer/") => (
            StatusCode::ACCEPTED,
            [(header::CACHE_CONTROL, "no-store")],
            axum::Json(computer_status(true)),
        )
            .into_response(),
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
        (Method::GET, _, repository) => {
            let mut instance = instance();
            match repository {
                Some(NO_SANDBOX) => {
                    instance["sandbox"] = Value::Null;
                    instance["sandboxId"] = Value::Null;
                }
                Some(BROKEN_STORE) => instance["sandboxId"] = json!(UNREADABLE_SANDBOX_ID),
                _ => {}
            }
            ([(header::CACHE_CONTROL, "no-store")], axum::Json(instance)).into_response()
        }
        _ => StatusCode::NO_CONTENT.into_response(),
    }
}

/// The Hub's routes behind the console's tools.
fn hub_routes(requests: Requests) -> Router {
    Router::new()
        .route("/factory/{profile}/instance", get(hub_route))
        .route("/factory/{profile}/watch", any(hub_route))
        .route("/lore/{profile}/repositories", any(hub_route))
        .route("/lore/{profile}/repositories/{id}/access", any(hub_route))
        .route("/lore/{profile}/groups", any(hub_route))
        .route("/lore/{profile}/groups/{name}/members", any(hub_route))
        .route("/selection/{profile}/{server}", any(hub_route))
        .route("/computer/status", get(hub_route))
        .route("/computer/restart", any(hub_route))
        .with_state(requests)
}

/// The query parameters of each request the fake log store received.
type Queries = Arc<Mutex<Vec<Vec<(String, String)>>>>;

/// What the fake log store holds for the sandbox, newest first, in the form
/// VictoriaLogs v1.53.0 returned the probe's records for console's query.
fn records() -> Vec<Value> {
    let role = "kubernetes.pod_labels.openshell.ai/boundary-role";
    vec![
        json!({"_time": "2026-10-22T10:00:05Z", "_msg": "error: process exited with code 101",
            "stream": "stderr", role: "workload"}),
        json!({"_time": "2026-10-22T10:00:03Z",
            "_msg": "2026-10-22T10:00:03.000Z OCSF NET:OPEN [MED] DENIED cargo(42) -> index.crates.io:443 [policy:- engine:opa]",
            "stream": "stdout", role: "supervisor"}),
        json!({"_time": "2026-10-22T10:00:02Z", "_msg": "   Compiling veoveo-mcp-time v0.1.0",
            "stream": "stderr", role: "workload"}),
        json!({"_time": "2026-10-22T10:00:01Z", "_msg": "Synced dev/jose/x@LATEST",
            "stream": "stdout", role: "workload"}),
        json!({"_time": "2026-10-22T10:00:00Z", "_msg": "x".repeat(5000),
            "stream": "stdout", role: "workload"}),
    ]
}

/// The fake log store's query endpoint: up to the query's `limit` records as JSON
/// lines, or VictoriaLogs' refusal of a query it cannot parse.
async fn log_query(State(queries): State<Queries>, uri: Uri) -> Response {
    let parameters: Vec<(String, String)> =
        url::form_urlencoded::parse(uri.query().unwrap_or_default().as_bytes())
            .into_owned()
            .collect();
    queries.lock().unwrap().push(parameters.clone());
    let query = parameters
        .iter()
        .find(|(name, _)| name == "query")
        .map(|(_, value)| value.clone())
        .unwrap_or_default();
    if query.contains(UNREADABLE_SANDBOX_ID) {
        return (
            StatusCode::BAD_REQUEST,
            "cannot parse `query` arg: unexpected token",
        )
            .into_response();
    }
    let limit: usize = query
        .split(" limit ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|limit| limit.parse().ok())
        .unwrap_or(usize::MAX);
    let body: String = records()
        .iter()
        .take(limit)
        .map(|record| format!("{record}\n"))
        .collect();
    body.into_response()
}

/// A fake log store on a loopback port, and the queries it receives.
async fn log_store() -> (url::Url, Queries) {
    let queries = Queries::default();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let base = url::Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let router = Router::new()
        .route("/select/logsql/query", get(log_query))
        .with_state(queries.clone());
    tokio::spawn(async move { axum::serve(listener, router).await });
    (base, queries)
}

/// The console on a mesh whose user holds `granted`, with the fake Hub routes and log
/// store.
async fn console(granted: &[ConsolePermission]) -> (RunningMesh, Requests, Queries) {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut mesh = TestMesh::bind().await;
    let requests = Requests::default();
    let (store, queries) = log_store().await;
    let calls = mesh.register(AUTHORIZATION.clone());
    let hub = Hub::new(mesh.base(), reqwest::Client::new()).unwrap();
    let logs = LogStore::new(&store, reqwest::Client::new()).unwrap();
    mesh.serve(
        testing::for_domain::<ConsoleMcp>()
            .calls(calls)
            .handler(move || Hosted::new(ConsoleMcp::new(hub.clone(), logs.clone())))
            .build(),
    );
    mesh.hub_route(hub_routes(requests.clone()));
    let mesh = mesh
        .granting(granted.iter().map(|permission| permission.name()))
        .start();
    (mesh, requests, queries)
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
    assert_eq!(
        names,
        [
            "computer",
            "create_group",
            "create_repository",
            "groups",
            "logs",
            "repository_access",
            "select",
            "set_group_member",
            "share_repository",
            "status",
            "watch"
        ]
    );
    for access in AUTHORIZATION.requirements.tools.values() {
        assert_eq!(
            *access,
            veoveo_hub_sdk::registration::RouteAccess::Permission(ConsolePermission::Use.name())
        );
    }
    // Without the console's permission no tool runs and the Hub hears nothing.
    let (denied, requests, _) = console(&[]).await;
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
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
    let status = call(
        &mesh,
        "status",
        json!({"repository": REPOSITORY, "branch": BRANCH, "wait": 600}),
    )
    .await
    .unwrap();
    assert_eq!(status["sandboxName"], SANDBOX);
    assert_eq!(status["workspace"], "vv-factory");
    assert_eq!(status["sandboxId"], SANDBOX_ID);
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
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
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
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
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
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
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

#[tokio::test]
async fn create_repository_creates_one_the_caller_owns_through_the_hub() {
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
    let created = call(&mesh, "create_repository", json!({"name": "hello"}))
        .await
        .unwrap();
    assert_eq!(
        created,
        json!({"id": CREATED, "name": "hello", "remote": format!("{LORE_REMOTE}/{CREATED}")})
    );

    // A creation the Hub did not finish names the repository, and the call that names
    // it finishes it.
    let unfinished = call(&mesh, "create_repository", json!({"name": UNREGISTERED}))
        .await
        .unwrap_err();
    assert_eq!(
        unfinished,
        format!(
            "creation of repository `{UNREGISTERED}`: the Hub answered 503 Service \
             Unavailable: Repository creation is unavailable: The repository was created in \
             Lore but not registered. Call again with this id to register it. (id `{CREATED}`)"
        )
    );
    let finished = call(
        &mesh,
        "create_repository",
        json!({"name": UNREGISTERED, "id": CREATED}),
    )
    .await
    .unwrap();
    assert_eq!(finished["id"], json!(CREATED));
    assert_eq!(finished["name"], json!(UNREGISTERED));

    let taken = call(&mesh, "create_repository", json!({"name": TAKEN}))
        .await
        .unwrap_err();
    assert!(
        taken.contains("409 Conflict: Lore holds a repository of this ID or name already"),
        "{taken}"
    );

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 4);
    for seen in &requests {
        assert_eq!(seen.method, Method::POST);
        assert_eq!(seen.path, format!("/lore/{MESH_PROFILE}/repositories"));
        assert!(seen.query.is_empty());
    }
    assert_eq!(requests[0].body, Some(json!({"name": "hello"})));
    assert_eq!(requests[1].body, Some(json!({"name": UNREGISTERED})));
    assert_eq!(
        requests[2].body,
        Some(json!({"name": UNREGISTERED, "id": CREATED}))
    );
    assert_as_the_caller(&requests);
}

#[tokio::test]
async fn sharing_reads_and_sets_a_repositorys_access_through_the_hub() {
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
    let path = format!("/lore/{MESH_PROFILE}/repositories/{CREATED}/access");

    let read = call(&mesh, "repository_access", json!({"repository": CREATED}))
        .await
        .unwrap();
    assert_eq!(read, access(CREATED));

    let shared = call(
        &mesh,
        "share_repository",
        json!({"repository": CREATED, "person": "ana@example.com", "access": "member"}),
    )
    .await
    .unwrap();
    assert_eq!(shared, access(CREATED));
    call(
        &mesh,
        "share_repository",
        json!({"repository": CREATED, "group": "flight", "access": "none"}),
    )
    .await
    .unwrap();

    let unknown = call(
        &mesh,
        "share_repository",
        json!({"repository": CREATED, "person": STRANGER, "access": "owner"}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        unknown,
        format!(
            "sharing of repository `{CREATED}`: the Hub answered 404 Not Found: Unknown \
             person: nobody with this email has signed in yet; they can be added once they \
             sign in"
        )
    );

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 4);
    for seen in &requests {
        assert_eq!(seen.path, path);
        assert!(seen.query.is_empty());
    }
    assert_eq!(requests[0].method, Method::GET);
    assert_eq!(requests[0].body, None);
    assert_eq!(requests[1].method, Method::PUT);
    assert_eq!(
        requests[1].body,
        Some(json!({"person": "ana@example.com", "access": "member"}))
    );
    assert_eq!(
        requests[2].body,
        Some(json!({"group": "flight", "access": "none"}))
    );
    assert_eq!(
        requests[3].body,
        Some(json!({"person": STRANGER, "access": "owner"}))
    );
    assert_as_the_caller(&requests);
}

#[tokio::test]
async fn groups_are_listed_created_and_changed_through_the_hub() {
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;

    let groups = call(&mesh, "groups", json!({})).await.unwrap();
    assert_eq!(groups, json!({"groups": [group("flight")]}));

    let created = call(&mesh, "create_group", json!({"name": "flight"}))
        .await
        .unwrap();
    assert_eq!(
        created,
        json!({"id": GROUP_ID, "name": "flight", "members": [{"email": SELF, "name": "Me", "role": "owner"}]})
    );

    let changed = call(
        &mesh,
        "set_group_member",
        json!({"group": "flight", "person": "ana@example.com", "role": "member"}),
    )
    .await
    .unwrap();
    assert_eq!(changed, group("flight"));

    let lockout = call(
        &mesh,
        "set_group_member",
        json!({"group": "flight", "person": SELF, "role": "none"}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        lockout,
        format!(
            "membership of `{SELF}` in group `flight`: the Hub answered 409 Conflict: The \
             group would have no owner: you would no longer own the group; make another \
             member its owner first"
        )
    );

    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].method, Method::GET);
    assert_eq!(requests[0].path, format!("/lore/{MESH_PROFILE}/groups"));
    assert_eq!(requests[1].method, Method::POST);
    assert_eq!(requests[1].path, format!("/lore/{MESH_PROFILE}/groups"));
    assert_eq!(requests[1].body, Some(json!({"name": "flight"})));
    for seen in &requests[2..] {
        assert_eq!(seen.method, Method::PUT);
        assert_eq!(
            seen.path,
            format!("/lore/{MESH_PROFILE}/groups/flight/members")
        );
    }
    assert_eq!(
        requests[2].body,
        Some(json!({"person": "ana@example.com", "role": "member"}))
    );
    assert_eq!(
        requests[3].body,
        Some(json!({"person": SELF, "role": "none"}))
    );
    assert_as_the_caller(&requests);
}

/// The expected query for the latest `limit` records of `sandbox_id`.
fn log_query_for(sandbox_id: &str, limit: usize) -> String {
    format!(
        "\"kubernetes.pod_labels.openshell.ai/boundary-pair\":=\"{sandbox_id}\" \
         (kubernetes.container_name:=\"agent\" or \
         (\"kubernetes.pod_labels.openshell.ai/boundary-role\":=\"supervisor\" OCSF DENIED)) \
         | sort by (_time desc) limit {limit} \
         | fields _time, _msg, stream, \"kubernetes.pod_labels.openshell.ai/boundary-role\""
    )
}

#[tokio::test]
async fn logs_read_the_sandbox_the_hubs_status_names_with_its_denials() {
    let (mesh, requests, queries) = console(&[ConsolePermission::Use]).await;
    let logs = call(
        &mesh,
        "logs",
        json!({"repository": REPOSITORY, "branch": BRANCH}),
    )
    .await
    .unwrap();
    assert_eq!(logs["sandboxName"], SANDBOX);
    assert_eq!(logs["sandboxId"], SANDBOX_ID);
    assert_eq!(logs["truncated"], false);
    let lines = logs["lines"].as_array().unwrap();
    let seen: Vec<(&str, &str)> = lines
        .iter()
        .map(|line| {
            (
                line["time"].as_str().unwrap(),
                line["source"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("2026-10-22T10:00:00Z", "stdout"),
            ("2026-10-22T10:00:01Z", "stdout"),
            ("2026-10-22T10:00:02Z", "stderr"),
            ("2026-10-22T10:00:03Z", "denied"),
            ("2026-10-22T10:00:05Z", "stderr"),
        ]
    );
    let long = lines[0]["text"].as_str().unwrap();
    assert!(long.len() <= 4096 && long.ends_with('…'), "{}", long.len());
    assert_eq!(lines[1]["text"], "Synced dev/jose/x@LATEST");
    assert!(
        lines[3]["text"]
            .as_str()
            .unwrap()
            .contains("DENIED cargo(42) -> index.crates.io:443")
    );

    // The status read without waiting is the access check, as the caller.
    let requests = requests.lock().unwrap().clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].path,
        format!("/factory/{MESH_PROFILE}/instance")
    );
    assert_eq!(
        query(&requests[0]),
        [
            ("repository", REPOSITORY),
            ("branch", BRANCH),
            ("wait", "0")
        ]
    );
    assert_as_the_caller(&requests);
    // One query by the sandbox's ID, one record past the default 200 lines.
    let queries = queries.lock().unwrap().clone();
    assert_eq!(
        queries,
        [vec![("query".to_owned(), log_query_for(SANDBOX_ID, 201))]]
    );
}

#[tokio::test]
async fn logs_start_at_since_and_keep_the_latest_lines_within_the_limit() {
    let (mesh, _, queries) = console(&[ConsolePermission::Use]).await;
    let logs = call(
        &mesh,
        "logs",
        json!({"repository": REPOSITORY, "branch": BRANCH,
            "since": "2026-10-22T10:00:02Z", "limit": 2}),
    )
    .await
    .unwrap();
    assert_eq!(logs["truncated"], true);
    let times: Vec<&str> = logs["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|line| line["time"].as_str().unwrap())
        .collect();
    assert_eq!(times, ["2026-10-22T10:00:03Z", "2026-10-22T10:00:05Z"]);
    assert_eq!(
        queries.lock().unwrap().clone(),
        [vec![
            ("query".to_owned(), log_query_for(SANDBOX_ID, 3)),
            ("start".to_owned(), "2026-10-22T10:00:02Z".to_owned()),
        ]]
    );

    // A limit above the bound reads at most 1000 lines.
    call(
        &mesh,
        "logs",
        json!({"repository": REPOSITORY, "branch": BRANCH, "limit": 5000}),
    )
    .await
    .unwrap();
    assert_eq!(
        queries.lock().unwrap().last().unwrap().clone(),
        [("query".to_owned(), log_query_for(SANDBOX_ID, 1001))]
    );
}

#[tokio::test]
async fn logs_need_the_hubs_answer_and_report_the_log_stores_refusal() {
    let (mesh, _, queries) = console(&[ConsolePermission::Use]).await;
    let refused = call(
        &mesh,
        "logs",
        json!({"repository": DENIED, "branch": BRANCH}),
    )
    .await
    .unwrap_err();
    assert!(
        refused.starts_with(&format!(
            "logs of `{DENIED}` branch `{BRANCH}`: the Hub answered 403 Forbidden"
        )),
        "{refused}"
    );
    // Without a sandbox there is nothing to read.
    let empty = call(
        &mesh,
        "logs",
        json!({"repository": NO_SANDBOX, "branch": BRANCH}),
    )
    .await
    .unwrap();
    assert_eq!(empty["sandboxId"], Value::Null);
    assert_eq!(empty["lines"], json!([]));
    assert!(queries.lock().unwrap().is_empty());

    let unreadable = call(
        &mesh,
        "logs",
        json!({"repository": BROKEN_STORE, "branch": BRANCH}),
    )
    .await
    .unwrap_err();
    assert_eq!(
        unreadable,
        format!(
            "logs of `{BROKEN_STORE}` branch `{BRANCH}`: the log store answered 400 Bad \
             Request: cannot parse `query` arg: unexpected token"
        )
    );
}

#[tokio::test]
async fn computer_reports_and_restarts_the_callers_computer_through_the_hub() {
    let (mesh, requests, _) = console(&[ConsolePermission::Use]).await;
    let status = call(&mesh, "computer", json!({})).await.unwrap();
    assert_eq!(status, computer_status(false));
    let restarting = call(&mesh, "computer", json!({"restart": true}))
        .await
        .unwrap();
    assert_eq!(restarting, computer_status(true));

    let requests = requests.lock().unwrap().clone();
    let calls: Vec<_> = requests
        .iter()
        .map(|seen| (seen.method.clone(), seen.path.clone(), seen.body.clone()))
        .collect();
    assert_eq!(
        calls,
        [
            (Method::GET, "/computer/status".to_owned(), None),
            (Method::POST, "/computer/restart".to_owned(), None),
        ]
    );
    assert_as_the_caller(&requests);
}

/// A Hub that answers only `GET /computer/version`, with `version`, as the Hub serves
/// it to anyone.
async fn versions(version: Arc<Mutex<Value>>) -> url::Url {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let base = url::Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let router = Router::new()
        .route(
            "/computer/version",
            get(|State(version): State<Arc<Mutex<Value>>>| async move {
                axum::Json(version.lock().unwrap().clone())
            }),
        )
        .with_state(version);
    tokio::spawn(async move { axum::serve(listener, router).await });
    base
}

#[tokio::test]
async fn the_computer_tool_names_the_current_version_and_follows_it() {
    use veoveo_mcp_console::mcp::computer;
    let _ = rustls::crypto::ring::default_provider().install_default();
    let version = Arc::new(Mutex::new(
        json!({"version": RUNNING, "summary": "The first edition"}),
    ));
    let hub = Hub::new(&versions(version.clone()).await, reqwest::Client::new()).unwrap();
    let logs = LogStore::new(
        &url::Url::parse("http://127.0.0.1:9").unwrap(),
        reqwest::Client::new(),
    )
    .unwrap();
    let listed = testing::TestHub::new(
        testing::for_domain::<ConsoleMcp>()
            .handler(move || Hosted::new(ConsoleMcp::new(hub.clone(), logs.clone())))
            .build(),
    )
    .granting([ConsolePermission::Use.name()]);
    let description = || async {
        listed.rpc("tools/list", json!({})).await["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "computer")
            .unwrap()["description"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(description().await, computer::UNKNOWN);

    let hub = Hub::new(&versions(version.clone()).await, reqwest::Client::new()).unwrap();
    assert!(computer::refresh(&hub).await.unwrap());
    let first = description().await;
    assert!(
        first.contains(&format!("`{RUNNING}`: The first edition")),
        "{first}"
    );
    assert!(
        first.contains(
            "Call this at the start of a session; if an update is available or was just \
             installed, tell the person what's new in a line or two; restart only when the \
             person asks."
        ),
        "{first}"
    );
    assert!(
        !computer::refresh(&hub).await.unwrap(),
        "the same version changes nothing"
    );

    *version.lock().unwrap() = json!({"version": CURRENT});
    assert!(computer::refresh(&hub).await.unwrap());
    let second = description().await;
    assert!(
        second.contains(&format!("The current version is `{CURRENT}`. With")),
        "{second}"
    );

    // A summary that ends with its own period ends the sentence once.
    *version.lock().unwrap() = json!({
        "version": RUNNING,
        "summary": "Your home ~ is now kept across restarts and updates."
    });
    assert!(computer::refresh(&hub).await.unwrap());
    let third = description().await;
    assert!(
        third.contains(&format!(
            "The current version is `{RUNNING}`: Your home ~ is now kept across restarts \
             and updates. With"
        )),
        "{third}"
    );
    assert!(!third.contains(".;") && !third.contains(".."), "{third}");
    assert!(!second.contains(RUNNING), "{second}");
}
