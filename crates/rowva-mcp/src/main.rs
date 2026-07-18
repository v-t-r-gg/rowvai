use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rmcp::{transport::stdio, ServiceExt};
use rowva_core::{ActorContext, ActorId, ActorType};
use rowva_mcp::{parse_capability, RowvaMcp, ServerIdentity};
use rowva_store_sqlite::SqliteApplication;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "rowva-mcp",
    version,
    about = "Accountable RowvAI MCP stdio adapter"
)]
struct Cli {
    #[command(subcommand)]
    command: CommandLine,
}
#[derive(Subcommand)]
enum CommandLine {
    Serve(ServeArgs),
}
#[derive(Clone, ValueEnum)]
enum ActorKind {
    Agent,
    Integration,
}
#[derive(Parser)]
struct ServeArgs {
    #[arg(long, env = "ROWVA_WORKSPACE")]
    workspace: PathBuf,
    #[arg(long, env = "ROWVA_ACTOR_ID")]
    actor_id: String,
    #[arg(long, env = "ROWVA_ACTOR_NAME")]
    actor_name: String,
    #[arg(long, env = "ROWVA_ACTOR_TYPE", default_value = "agent")]
    actor_type: ActorKind,
    #[arg(long, env = "ROWVA_ACTOR_VERSION")]
    actor_version: Option<String>,
    #[arg(
        long = "capability",
        env = "ROWVA_CAPABILITIES",
        value_delimiter = ',',
        required = true
    )]
    capabilities: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logging();
    match cli.command {
        CommandLine::Serve(args) => serve(args).await,
    }
}
fn init_logging() {
    let filter =
        EnvFilter::try_from_env("ROWVA_LOG").unwrap_or_else(|_| EnvFilter::new("rowva_mcp=info"));
    let json = std::env::var("ROWVA_LOG_FORMAT").is_ok_and(|v| v == "json");
    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    }
}
async fn serve(args: ServeArgs) -> Result<()> {
    let path = std::fs::canonicalize(&args.workspace)
        .with_context(|| "workspace path does not exist or cannot be resolved")?;
    if !path.is_file() {
        anyhow::bail!("workspace must be a regular file");
    }
    if args.actor_name.trim().is_empty() {
        anyhow::bail!("actor name is required");
    }
    let raw = if args.actor_id.starts_with("act_") {
        args.actor_id
    } else {
        format!("act_{}", args.actor_id)
    };
    let actor_id = ActorId::from_string(raw).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let capabilities = args
        .capabilities
        .iter()
        .map(|v| parse_capability(v).map_err(anyhow::Error::msg))
        .collect::<Result<Vec<_>>>()?;
    let session_id = format!("ses_{}", Uuid::new_v4().simple());
    let actor = ActorContext {
        id: actor_id,
        actor_type: match args.actor_type {
            ActorKind::Agent => ActorType::Agent,
            ActorKind::Integration => ActorType::Integration,
        },
        display_name: args.actor_name,
        capabilities,
        actor_version: args.actor_version.clone(),
        human_principal_id: None,
        client_name: Some("mcp-stdio".into()),
        session_id: Some(session_id.clone()),
    };
    let application = SqliteApplication::open(&path)
        .map_err(|e| anyhow::anyhow!("workspace startup failed: {e}"))?;
    let workspace = application.workspace_id().clone();
    let identity = ServerIdentity {
        actor: actor.clone(),
        actor_version: args.actor_version,
        session_id,
    };
    info!(event="mcp.server.start",workspace_id=%workspace,actor_id=%actor.id,actor_type=?actor.actor_type,session_id=%identity.session_id,transport="stdio");
    let service = RowvaMcp::new(application, identity).serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
