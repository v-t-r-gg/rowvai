use clap::{Parser, Subcommand};
use rowva_core::*;
use rowva_store_sqlite::SqliteApplication;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "rowva-eval",
    about = "RowvAI non-authoritative shadow evaluation CLI",
    long_about = "Evaluates frozen CRM decisions. Readiness is advisory and never grants capabilities, changes policy, or executes CRM mutations."
)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Top,
}
#[derive(Subcommand)]
enum Top {
    Fixture {
        #[command(subcommand)]
        command: Fixture,
    },
    Case {
        #[command(subcommand)]
        command: Case,
    },
    Candidate {
        #[command(subcommand)]
        command: Candidate,
    },
    Outcome {
        #[command(subcommand)]
        command: Outcome,
    },
    Replay {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        case_id: String,
        #[arg(long)]
        candidate_id: String,
    },
    Report {
        #[arg(long)]
        workspace: PathBuf,
    },
}
#[derive(Subcommand)]
enum Fixture {
    Run { fixture: PathBuf },
}
#[derive(Subcommand)]
enum Case {
    Create {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    Show {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        case_id: String,
    },
    Export {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        case_id: String,
    },
}
#[derive(Subcommand)]
enum Candidate {
    Submit {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
}
#[derive(Subcommand)]
enum Outcome {
    Record {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeFile {
    case_id: EvaluationCaseId,
    outcome: HumanEvaluationOutcomeInput,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T, RowvaError> {
    let bytes = fs::read(path)
        .map_err(|_| RowvaError::validation("input_read_failed", "unable to read input file"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| RowvaError::validation("invalid_input", "input is not valid strict JSON"))
}
fn open(path: &Path) -> Result<SqliteApplication, RowvaError> {
    SqliteApplication::open(path)
}
fn output<T: serde::Serialize>(value: &T, json_mode: bool) -> Result<(), RowvaError> {
    let encoded = if json_mode {
        serde_json::to_string(value)
    } else {
        serde_json::to_string_pretty(value)
    }
    .map_err(|_| RowvaError::Internal {
        code: "output_serialization_failed".into(),
    })?;
    println!("{encoded}");
    Ok(())
}
fn run(cli: Cli) -> Result<(), RowvaError> {
    match cli.command {
        Top::Case {
            command: Case::Create { workspace, input },
        } => {
            let mut app = open(&workspace)?;
            output(&app.create_evaluation_case(read(&input)?)?, cli.json)
        }
        Top::Case {
            command: Case::Show { workspace, case_id },
        } => output(
            &open(&workspace)?.get_evaluation_case(&EvaluationCaseId::from_string(case_id)?)?,
            cli.json,
        ),
        Top::Case {
            command: Case::Export { workspace, case_id },
        } => output(
            &open(&workspace)?.export_evaluation_case(&EvaluationCaseId::from_string(case_id)?)?,
            true,
        ),
        Top::Candidate {
            command: Candidate::Submit { workspace, input },
        } => {
            let mut app = open(&workspace)?;
            output(&app.submit_evaluation_candidate(read(&input)?)?, cli.json)
        }
        Top::Outcome {
            command: Outcome::Record { workspace, input },
        } => {
            let value: OutcomeFile = read(&input)?;
            let mut app = open(&workspace)?;
            output(
                &app.record_evaluation_outcome(&value.case_id, value.outcome)?,
                cli.json,
            )
        }
        Top::Replay {
            workspace,
            case_id,
            candidate_id,
        } => output(
            &open(&workspace)?.replay_evaluation_candidate(
                &EvaluationCaseId::from_string(case_id)?,
                &EvaluationCandidateId::from_string(candidate_id)?,
            )?,
            cli.json,
        ),
        Top::Report { workspace } => output(&open(&workspace)?.evaluation_report()?, cli.json),
        Top::Fixture {
            command: Fixture::Run { fixture },
        } => {
            let value: Value = read(&fixture)?;
            let cases = value
                .get("cases")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    RowvaError::validation("invalid_fixture", "fixture requires a cases array")
                })?;
            output(
                &json!({"fixture_version":value.get("fixture_version"),"synthetic":true,"cases":cases.len(),"network_required":false,"authoritative_mutations":0}),
                cli.json,
            )
        }
    }
}
fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!(
                "{}",
                serde_json::to_string(&error)
                    .unwrap_or_else(|_| "{\"category\":\"internal\"}".into())
            );
            ExitCode::FAILURE
        }
    }
}
