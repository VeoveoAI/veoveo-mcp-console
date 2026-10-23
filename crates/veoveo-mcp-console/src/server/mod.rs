//! The hosted process: register with the Hub, connect with the registration's
//! database login, hold a process instance, then serve MCP until SIGTERM or Ctrl-C or
//! until the instance loses its lock. The server keeps no state of its own; the
//! process instance is what the Hub reads its health from. Process stop preserves
//! native resume credentials.
mod config;
mod registration;

use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use veoveo_hub_sdk::{
    PublicDeployment, TelemetryGuard,
    hosting::{Hosted, HostedServer, ServerCalls},
    init_server_telemetry,
    registration::{ClientMetadata, client::Registration, statement::ServerKeyPair},
};
use veoveo_store::{Instance, PlatformStore, StoreConfig};

use crate::mcp::{self, ConsoleMcp, Hub, LogStore};

use config::Args;
use registration::Registered;

/// What the tools' token exchanges sign with: the admitted metadata and its key.
struct Signing<'a> {
    metadata: &'a ClientMetadata,
    key: &'a ServerKeyPair,
}

pub async fn run() -> Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let _ = dotenvy::dotenv();
    let args = Args::parse();
    let deployment = args.public_deployment()?;
    let declaration = mcp::registration(&deployment, args.deployment_id)?;
    mcp::REGISTRATION
        .set(declaration.clone())
        .map_err(|_| anyhow::anyhow!("the registration was declared twice"))?;
    mcp::force_setup();
    if args.describe_registration {
        let metadata = registration::prepare(&args, declaration)?;
        serde_json::to_writer_pretty(std::io::stdout().lock(), &metadata)?;
        println!();
        return Ok(());
    }
    let _telemetry: TelemetryGuard =
        init_server_telemetry(crate::component(), "info,veoveo_mcp_console=debug")?;
    let Registered {
        client,
        registration,
        metadata,
        key,
        http,
    } = registration::register(&args, declaration).await?;
    // Process stop preserves saved native registration; explicit retirement and the
    // Hub's existing orphan grace remain separate.
    let issued = registration.clone();
    let mut served = None;
    let maintained = client
        .maintain(registration, async {
            served = Some(
                serve(
                    &args,
                    &deployment,
                    &issued,
                    Signing {
                        metadata: &metadata,
                        key: &key,
                    },
                    http,
                )
                .await,
            );
        })
        .await;
    if let Err(error) = maintained {
        tracing::warn!(%error, "registration lifecycle failed");
    }
    served.context("the registration ended before serving finished")?
}

/// Connect with the registration's database login, hold a process instance and serve
/// until SIGTERM or Ctrl-C, or until the process instance loses its lock.
async fn serve(
    args: &Args,
    deployment: &PublicDeployment,
    registration: &Registration,
    signing: Signing<'_>,
    http: reqwest::Client,
) -> Result<()> {
    // The tools call the Hub's routes for their caller with tokens exchanged at the
    // Hub's token endpoint, signed with the registration's key.
    let calls = ServerCalls::discover(registration, signing.metadata, signing.key, http.clone())
        .await
        .context("discovering the Hub's token endpoint")?;
    let hub = Hub::new(
        args.hub_issuer
            .as_ref()
            .context("serving requires a Hub issuer")?,
        http.clone(),
    )?;
    let logs = LogStore::new(&args.log_store_url, http.clone())?;
    // The `computer` tool's description follows the current Computer version.
    let follower = tokio::spawn(mcp::computer::follow(hub.clone()));
    let grant = registration
        .database()
        .context("the Hub issued no database login")?;
    let store = PlatformStore::connect(
        StoreConfig::builder(grant.url.as_str(), grant.password.clone())
            .username(grant.username.as_str())
            .build()?,
    )
    .await
    .context("connecting to the installation database")?;
    let instance = Arc::new(
        Instance::start(&store, &crate::server())
            .await
            .context("starting the process instance")?,
    );
    let ready_instance = instance.clone();
    let hosted = HostedServer::for_domain::<ConsoleMcp>()
        .deployment(deployment, args.allow_loopback_hosts)?
        .allowed_hosts(args.allowed_hosts.iter().cloned())
        .internal_trust(registration.access_tokens(), http.clone())?
        .calls(calls)
        .handler(move || Hosted::new(ConsoleMcp::new(hub.clone(), logs.clone())))
        // The SDK's anonymous /readyz is installed only after native registration and
        // the database connection succeeded.
        .readiness(move || {
            let instance = ready_instance.clone();
            async move { !instance.is_lost() }
        })
        .build();
    let served = tokio::select! {
        served = hosted.serve(SocketAddr::from(([0, 0, 0, 0], args.port))) => served,
        () = instance.lost() => Err(anyhow::anyhow!("the process instance lost its lock")),
    };
    follower.abort();
    if let Err(error) = instance.shutdown().await {
        tracing::warn!(%error, "the process instance did not stop cleanly");
    }
    store.close().await;
    served
}
