//! One typed route table binds the tools, their outputs, discovery and permissions.
//! Each tool calls the Hub for its caller, at the profile the caller came through, and
//! reports the Hub's answer; a refusal is the tool's error.
mod declarations;
pub mod hub;
mod requirements;
mod setup;

use std::{sync::LazyLock, time::Duration};

use rmcp::{RoleServer, service::RequestContext};
use veoveo_hub_sdk::{
    HubProfileId,
    client::HubClient,
    hosting::{ClientContext, DomainServer, RoutePolicy, Routes, ToolError, hub_identity},
    server_contract::McpServerSetup,
};

use crate::contract::{
    ConsolePermission, InstanceStatus, SelectRequest, Selection, StatusRequest, WatchRequest,
    Watching,
};
pub use declarations::{REGISTRATION, registration};
pub use hub::Hub;
pub use requirements::AUTHORIZATION;
pub use setup::ConsoleContract;
use setup::SERVER_SETUP;

pub fn force_setup() {
    LazyLock::force(&SERVER_SETUP);
}

/// The console's tools over the Hub.
#[derive(Clone)]
pub struct ConsoleMcp {
    hub: Hub,
}

impl ConsoleMcp {
    pub fn new(hub: Hub) -> Self {
        Self { hub }
    }

    async fn status(
        &self,
        request: StatusRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<InstanceStatus, ToolError> {
        let (client, profile) = caller(&context)?;
        let wait = Duration::from_secs(request.wait.unwrap_or_default().into());
        self.hub
            .status(
                &client,
                &profile,
                &request.repository,
                &request.branch,
                wait,
            )
            .await
            .map_err(|error| {
                refused(
                    format_args!(
                        "status of `{}` branch `{}`",
                        request.repository, request.branch
                    ),
                    error,
                )
            })
    }

    async fn select(
        &self,
        request: SelectRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<Selection, ToolError> {
        let (client, profile) = caller(&context)?;
        self.hub
            .select(
                &client,
                &profile,
                &request.server,
                request.branch.as_deref(),
            )
            .await
            .map_err(|error| refused(format_args!("selection of `{}`", request.server), error))?;
        Ok(Selection {
            server: request.server,
            branch: request.branch,
        })
    }

    async fn watch(
        &self,
        request: WatchRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<Watching, ToolError> {
        let (client, profile) = caller(&context)?;
        self.hub
            .watch(&client, &profile, &request.repository)
            .await
            .map_err(|error| refused(format_args!("watch of `{}`", request.repository), error))?;
        Ok(Watching {
            repository: request.repository,
        })
    }
}

/// The calls for this request's caller and the profile it came through.
fn caller(context: &RequestContext<RoleServer>) -> Result<(HubClient, HubProfileId), ToolError> {
    let identity = hub_identity(context)?;
    Ok((context.client()?, identity.profile))
}

/// A tool error naming what the Hub refused and why.
fn refused(what: std::fmt::Arguments<'_>, error: impl std::fmt::Display) -> ToolError {
    ToolError::Rejected(format!("{what}: {error}"))
}

fn policy() -> RoutePolicy {
    RoutePolicy::permission(ConsolePermission::Use).expect("the console permission")
}

static ROUTES: LazyLock<Routes<ConsoleMcp>> = LazyLock::new(|| {
    Routes::<ConsoleMcp>::builder(policy())
        .tool::<StatusRequest, _, _>(
            "Why a branch of a repository serves or does not: the revision the Hub applied \
             and the one serving, the sandbox's phase, exit code, restart count and \
             conditions, the Hub's last failure and any warning. With `wait`, answers as \
             soon as the applied revision serves or fails, after at most 60 seconds. After \
             a push, call it with `wait`; on failure, read `logs`.",
            policy(),
            |domain, request, context| async move { domain.status(request, context).await },
        )
        .tool::<SelectRequest, _, _>(
            "Serve yourself `server` from the instance of `branch` in every chat, or from \
             the base branch when `branch` is null. Your tool list changes with it.",
            policy(),
            |domain, request, context| async move { domain.select(request, context).await },
        )
        .tool::<WatchRequest, _, _>(
            "Have the Hub watch a repository as you, so every branch with an executable \
             `openshell/run` is built and served when someone pushes it.",
            policy(),
            |domain, request, context| async move { domain.watch(request, context).await },
        )
        .calls_hub()
        .build()
});

impl DomainServer for ConsoleMcp {
    type Contract = ConsoleContract;
    fn setup() -> &'static McpServerSetup<ConsoleContract> {
        &SERVER_SETUP
    }
    fn routes() -> &'static Routes<Self> {
        &ROUTES
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_setup_and_documents_admit() {
        std::sync::LazyLock::force(&super::SERVER_SETUP);
        assert!(super::setup::SERVER_DOCS.agent_manual().is_some());
    }

    #[test]
    fn the_routes_declare_calls_to_the_hub_and_no_uses() {
        let authorization = super::ROUTES.authorization();
        assert!(authorization.calls_hub);
        assert!(authorization.uses.is_empty());
        assert_eq!(
            authorization
                .permissions
                .iter()
                .map(|permission| permission.permission.as_str())
                .collect::<Vec<_>>(),
            ["console.use"]
        );
    }
}
