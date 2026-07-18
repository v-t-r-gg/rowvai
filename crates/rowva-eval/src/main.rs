use clap::{Parser, Subcommand, ValueEnum};
use rowva_core::*;
use rowva_store_sqlite::SqliteApplication;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
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
    Dataset {
        #[command(subcommand)]
        command: DatasetCommand,
    },
    Analysis {
        #[command(subcommand)]
        command: AnalysisCommand,
    },
}
#[derive(Subcommand)]
enum Fixture {
    Run { fixture: PathBuf },
    QualityRun { fixture: PathBuf },
}
#[derive(Subcommand)]
enum Case {
    List {
        #[arg(long)]
        workspace: PathBuf,
    },
    Invalidate {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
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
enum DatasetCommand {
    Create {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    List {
        #[arg(long)]
        workspace: PathBuf,
    },
    Show {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        dataset_id: String,
    },
    Quality {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        dataset_id: String,
        #[arg(long)]
        actor_id: String,
        #[arg(long)]
        actor_version: String,
    },
    Export {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        dataset_id: String,
        #[arg(long, value_enum, default_value = "metadata")]
        profile: ExportProfileArg,
        #[arg(long)]
        output: Option<PathBuf>,
    },
}
#[derive(Subcommand)]
enum AnalysisCommand {
    Run {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    List {
        #[arg(long)]
        workspace: PathBuf,
    },
    Show {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        analysis_id: String,
    },
}
#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
enum ExportProfileArg {
    Metadata,
    #[value(name = "full_local", alias = "full-local")]
    FullLocal,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureFile {
    fixture_version: u16,
    synthetic: bool,
    cases: Vec<FixtureCase>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    name: String,
    human: String,
    candidates: Vec<FixtureCandidate>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureCandidate {
    actor_version: String,
    decision: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    revision_offset: i64,
    expected_verdict: EvaluationVerdict,
    expected_validation: CandidateValidationStatus,
    expected_eligible: bool,
}
#[derive(Debug, Serialize)]
struct FixtureSummary {
    fixture_version: u16,
    scenario_count: usize,
    passed: usize,
    failed: usize,
    scenarios: Vec<FixtureScenarioResult>,
}
#[derive(Debug, Serialize)]
struct FixtureScenarioResult {
    name: String,
    passed: bool,
    expected_verdicts: Vec<EvaluationVerdict>,
    actual_verdicts: Vec<EvaluationVerdict>,
    replay_deterministic: bool,
    mutation_safe: bool,
    candidate_created_no_operation: bool,
    candidate_created_no_approval: bool,
    candidate_preserved_record_revision: bool,
    candidate_preserved_schema_revision: bool,
    human_operation_delta_correct: bool,
    replay_created_no_operation: bool,
    replay_created_no_approval: bool,
    replay_preserved_evaluation_evidence: bool,
    replay_preserved_record_revision: bool,
    replay_preserved_schema_revision: bool,
    versions_separate: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityFixtureFile {
    fixture_protocol_version: u16,
    synthetic: bool,
    scenarios: Vec<QualityFixtureExpectation>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QualityFixtureExpectation {
    name: String,
    expected: bool,
}
#[derive(Debug, Serialize)]
struct QualityFixtureSummary {
    fixture_protocol_version: u16,
    scenarios_attempted: usize,
    passed: usize,
    failed: usize,
    scenarios: Vec<QualityFixtureResult>,
    privacy_checks: Value,
    integrity_checks: Value,
}
#[derive(Debug, Serialize)]
struct QualityFixtureResult {
    name: String,
    expected: bool,
    actual: bool,
    passed: bool,
}

#[derive(Clone, Copy)]
enum QualityDecision {
    NoChange,
    Change,
    Abstain,
    Invalid,
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
fn write_sensitive_export(path: &Path, value: &EvaluationDatasetExport) -> Result<(), RowvaError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| {
        RowvaError::validation("export_serialization_failed", "unable to serialize export")
    })?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|_| {
                RowvaError::validation(
                    "export_write_failed",
                    "full-local export output must be a new writable file",
                )
            })?;
        file.write_all(&bytes).map_err(|_| {
            RowvaError::validation("export_write_failed", "unable to write full-local export")
        })?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes).map_err(|_| {
            RowvaError::validation("export_write_failed", "unable to write full-local export")
        })?;
    }
    Ok(())
}
fn commit(app: &mut SqliteApplication, command: Command) -> Result<OperationResponse, RowvaError> {
    app.execute(OperationRequest::new(ActorContext::local_user(), command))
}
fn run_fixture_file(file: FixtureFile) -> Result<FixtureSummary, RowvaError> {
    if file.fixture_version != 1 || !file.synthetic {
        return Err(RowvaError::validation(
            "unsupported_fixture",
            "fixture must be synthetic version 1",
        ));
    }
    let mut scenarios = Vec::new();
    for scenario in file.cases {
        let dir = tempfile::tempdir().map_err(|_| RowvaError::Internal {
            code: "fixture_tempdir_failed".into(),
        })?;
        let path = dir.path().join("fixture.rowva");
        let mut app = SqliteApplication::create(&path, "Synthetic Fixture")?;
        commit(
            &mut app,
            Command::CreateObject {
                display_name: "Deals".into(),
                key: Some("deals".into()),
            },
        )?;
        let object = app.list_objects()?.remove(0).id;
        commit(
            &mut app,
            Command::CreateField {
                object_id: object.clone(),
                display_name: "Stage".into(),
                key: Some("stage".into()),
                kind: FieldKind::Enum(EnumConfig {
                    options: vec!["Discovery".into(), "Qualified".into(), "Negotiation".into()],
                }),
                required: true,
                unique: false,
            },
        )?;
        commit(
            &mut app,
            Command::CreateField {
                object_id: object.clone(),
                display_name: "Notes".into(),
                key: Some("notes".into()),
                kind: FieldKind::Text(TextConfig::default()),
                required: false,
                unique: false,
            },
        )?;
        let fields = app.list_fields(&object)?;
        let stage = fields
            .iter()
            .find(|f| f.key == "stage")
            .ok_or_else(|| RowvaError::Internal {
                code: "fixture_stage_missing".into(),
            })?
            .id
            .clone();
        let notes = fields
            .iter()
            .find(|f| f.key == "notes")
            .ok_or_else(|| RowvaError::Internal {
                code: "fixture_notes_missing".into(),
            })?
            .id
            .clone();
        let mut values = HashMap::new();
        values.insert(stage.clone(), json!("Discovery"));
        let created = commit(
            &mut app,
            Command::CreateRecord {
                object_id: object.clone(),
                record_id: None,
                values,
            },
        )?;
        let record = RecordId::from_string(
            created
                .result
                .and_then(|v| {
                    v.get("record_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .ok_or_else(|| RowvaError::Internal {
                    code: "fixture_record_missing".into(),
                })?,
        )?;
        let case = app.create_evaluation_case(EvaluationCaseCreate {
            protocol_version: 1,
            target_object_id: object.clone(),
            target_record_id: record.clone(),
            target_stage_field_id: stage.clone(),
            relevant_field_ids: vec![notes.clone()],
            input: DealStageQualificationInputV1 {
                source_meeting_id: Some(format!("mtg_{}", scenario.name)),
                meeting_evidence: json!({"synthetic":true,"scenario":scenario.name.clone()}),
                current_stage: json!("Discovery"),
            },
            creator: ActorContext::local_user(),
        })?;
        let export = app.export_evaluation_case(&case.id)?;
        if evaluation_bundle_digest(&export.bundle)? != export.bundle_digest {
            return Err(RowvaError::validation(
                "fixture_digest_mismatch",
                "exported fixture digest is not reproducible",
            ));
        }
        let before = app.get_record(&object, &record)?;
        let operations = app.list_operations(100)?.len();
        let approvals = app.list_approvals(100)?.len();
        let mut submitted = Vec::new();
        let mut expected = Vec::new();
        let mut statuses = Vec::new();
        for spec in &scenario.candidates {
            let actor = ActorContext {
                id: ActorId::from_string("act_fixture_agent")?,
                actor_type: ActorType::Agent,
                display_name: "Synthetic Fixture Agent".into(),
                capabilities: vec![Capability::OperationsApprove],
                actor_version: Some(spec.actor_version.clone()),
                human_principal_id: None,
                client_name: Some("fixture".into()),
                session_id: Some(format!("run_{}", scenario.name)),
            };
            let decision = match spec.decision.as_str() {
                "no_change" => ShadowDecision::NoChange {
                    reason: Some("synthetic decision".into()),
                },
                "abstain" => ShadowDecision::Abstain {
                    reason: "synthetic abstention".into(),
                },
                _ => {
                    let target_record = if spec.target.as_deref() == Some("wrong_record") {
                        RecordId::new()
                    } else {
                        record.clone()
                    };
                    let field_id = if spec.target.as_deref() == Some("wrong_field") {
                        FieldId::new()
                    } else {
                        stage.clone()
                    };
                    ShadowDecision::ProposeStageUpdate {
                        proposal: StageUpdateProposalV1 {
                            object_id: object.clone(),
                            record_id: target_record,
                            field_id,
                            expected_revision: RecordRevision(1_i64 + spec.revision_offset),
                            value: json!(spec.value.clone().unwrap_or_else(|| "Qualified".into())),
                        },
                    }
                }
            };
            let candidate = app.submit_evaluation_candidate(CandidateImport {
                protocol_version: 1,
                case_id: case.id.clone(),
                bundle_digest: case.bundle_digest.clone(),
                actor,
                decision,
                reason: Some("synthetic fixture".into()),
                confidence: None,
            })?;
            statuses.push(
                candidate.validation_status == spec.expected_validation
                    && candidate.eligible_for_metrics == spec.expected_eligible,
            );
            expected.push(spec.expected_verdict);
            submitted.push(candidate);
        }
        let candidate_record = app.get_record(&object, &record)?;
        let candidate_created_no_operation = app.list_operations(100)?.len() == operations;
        let candidate_created_no_approval = app.list_approvals(100)?.len() == approvals;
        let candidate_preserved_record_revision = candidate_record.revision == before.revision;
        let candidate_preserved_schema_revision =
            app.schema_revision()? == case.base_schema_revision;
        let human_changes_stage = scenario.human != "Discovery";
        let outcome = if scenario.human == "Discovery" {
            HumanEvaluationOutcomeInput::NoChange {
                actor: ActorContext::local_user(),
                reason: Some("synthetic reference label".into()),
            }
        } else {
            let mut change = HashMap::new();
            change.insert(stage.clone(), json!(scenario.human));
            let response = commit(
                &mut app,
                Command::UpdateRecord {
                    object_id: object.clone(),
                    record_id: record.clone(),
                    values: change,
                    expected_revision: Some(RecordRevision(1)),
                },
            )?;
            HumanEvaluationOutcomeInput::CommittedOperation {
                operation_id: response.operation_id,
            }
        };
        let results = app.record_evaluation_outcome(&case.id, outcome)?;
        let actual = results.iter().map(|r| r.verdict).collect::<Vec<_>>();
        let expected_human_operation_delta = usize::from(human_changes_stage);
        let human_operation_delta_correct =
            app.list_operations(100)?.len() == operations + expected_human_operation_delta;
        let replay_operations = app.list_operations(100)?.len();
        let replay_approvals = app.list_approvals(100)?.len();
        let replay_record_revision = app.get_record(&object, &record)?.revision;
        let replay_schema_revision = app.schema_revision()?;
        let replay_report = app.evaluation_report()?;
        let replay_case = app.get_evaluation_case(&case.id)?;
        let replays = submitted
            .iter()
            .map(|c| app.replay_evaluation_candidate(&case.id, &c.id))
            .collect::<Result<Vec<_>, _>>()?;
        let replay_deterministic = replays.iter().all(|r| r.deterministic_match);
        let replay_created_no_operation = app.list_operations(100)?.len() == replay_operations;
        let replay_created_no_approval = app.list_approvals(100)?.len() == replay_approvals;
        let replay_preserved_record_revision =
            app.get_record(&object, &record)?.revision == replay_record_revision;
        let replay_preserved_schema_revision = app.schema_revision()? == replay_schema_revision;
        let replay_preserved_evaluation_evidence = app.evaluation_report()? == replay_report
            && app.get_evaluation_case(&case.id)? == replay_case;
        let expected_live = if scenario.human == "Discovery" {
            json!("Discovery")
        } else {
            json!(scenario.human)
        };
        let mutation_safe = candidate_created_no_operation
            && candidate_created_no_approval
            && candidate_preserved_record_revision
            && candidate_preserved_schema_revision
            && human_operation_delta_correct
            && replay_created_no_operation
            && replay_created_no_approval
            && replay_preserved_evaluation_evidence
            && replay_preserved_record_revision
            && replay_preserved_schema_revision
            && app.get_record(&object, &record)?.values.get(&stage) == Some(&expected_live);
        let versions_separate = if submitted.len() > 1 {
            let report = app.evaluation_report()?;
            report.len() == submitted.len()
        } else {
            true
        };
        let passed = statuses.iter().all(|v| *v)
            && actual == expected
            && replay_deterministic
            && mutation_safe
            && versions_separate;
        scenarios.push(FixtureScenarioResult {
            name: scenario.name,
            passed,
            expected_verdicts: expected,
            actual_verdicts: actual,
            replay_deterministic,
            mutation_safe,
            candidate_created_no_operation,
            candidate_created_no_approval,
            candidate_preserved_record_revision,
            candidate_preserved_schema_revision,
            human_operation_delta_correct,
            replay_created_no_operation,
            replay_created_no_approval,
            replay_preserved_evaluation_evidence,
            replay_preserved_record_revision,
            replay_preserved_schema_revision,
            versions_separate,
        });
    }
    let passed = scenarios.iter().filter(|s| s.passed).count();
    let failed = scenarios.len() - passed;
    Ok(FixtureSummary {
        fixture_version: file.fixture_version,
        scenario_count: scenarios.len(),
        passed,
        failed,
        scenarios,
    })
}

fn quality_agent() -> Result<ActorContext, RowvaError> {
    Ok(ActorContext {
        id: ActorId::from_string("act_quality_fixture")?,
        actor_type: ActorType::Agent,
        display_name: "SYNTHETIC_PRIVATE_DISPLAY_NAME".into(),
        capabilities: vec![Capability::OperationsApprove],
        actor_version: Some("quality-v1".into()),
        human_principal_id: None,
        client_name: Some("SYNTHETIC_PRIVATE_CLIENT".into()),
        session_id: Some("SYNTHETIC_PRIVATE_SESSION".into()),
    })
}

#[allow(clippy::too_many_arguments)]
fn create_quality_case(
    app: &mut SqliteApplication,
    object: &ObjectId,
    stage: &FieldId,
    notes: &FieldId,
    index: usize,
    source_meeting_id: &str,
    evidence_tag: &str,
    decision: Option<QualityDecision>,
    confidence: Option<f64>,
    human_change: bool,
    existing_record: Option<RecordId>,
) -> Result<(EvaluationCase, RecordId, bool), RowvaError> {
    let record = if let Some(record) = existing_record {
        record
    } else {
        let mut values = HashMap::new();
        values.insert(stage.clone(), json!("Discovery"));
        values.insert(notes.clone(), json!("SYNTHETIC_PRIVATE_RECORD_VALUE"));
        let response = commit(
            app,
            Command::CreateRecord {
                object_id: object.clone(),
                record_id: None,
                values,
            },
        )?;
        RecordId::from_string(
            response
                .result
                .and_then(|v| {
                    v.get("record_id")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .ok_or_else(|| {
                    RowvaError::validation(
                        "quality_fixture_record_missing",
                        "fixture record result missing",
                    )
                })?,
        )?
    };
    let case = app.create_evaluation_case(EvaluationCaseCreate {
        protocol_version: 1,
        target_object_id: object.clone(),
        target_record_id: record.clone(),
        target_stage_field_id: stage.clone(),
        relevant_field_ids: vec![notes.clone()],
        input: DealStageQualificationInputV1 {
            source_meeting_id: Some(source_meeting_id.into()),
            meeting_evidence: json!({"text":"SYNTHETIC_PRIVATE_MEETING_TEXT","tag":evidence_tag}),
            current_stage: json!("Discovery"),
        },
        creator: ActorContext::local_user(),
    })?;
    let before = app.get_record(object, &record)?;
    if let Some(decision) = decision {
        let shadow = match decision {
            QualityDecision::NoChange => ShadowDecision::NoChange {
                reason: Some("SYNTHETIC_PRIVATE_REASON".into()),
            },
            QualityDecision::Change => ShadowDecision::ProposeStageUpdate {
                proposal: StageUpdateProposalV1 {
                    object_id: object.clone(),
                    record_id: record.clone(),
                    field_id: stage.clone(),
                    expected_revision: case.base_record_revision,
                    value: json!("Qualified"),
                },
            },
            QualityDecision::Abstain => ShadowDecision::Abstain {
                reason: "SYNTHETIC_PRIVATE_REASON".into(),
            },
            QualityDecision::Invalid => ShadowDecision::ProposeStageUpdate {
                proposal: StageUpdateProposalV1 {
                    object_id: object.clone(),
                    record_id: record.clone(),
                    field_id: FieldId::new(),
                    expected_revision: case.base_record_revision,
                    value: json!("Qualified"),
                },
            },
        };
        let imported = app.submit_evaluation_candidate(CandidateImport {
            protocol_version: 1,
            case_id: case.id.clone(),
            bundle_digest: case.bundle_digest.clone(),
            actor: quality_agent()?,
            decision: shadow,
            reason: Some("SYNTHETIC_PRIVATE_REASON".into()),
            confidence,
        })?;
        if !imported.actor.capabilities.is_empty() {
            return Err(RowvaError::validation(
                "quality_fixture_authority_changed",
                "candidate capabilities were not sanitized",
            ));
        }
    }
    let shadow_safe = app.get_record(object, &record)? == before;
    if human_change {
        let mut values = HashMap::new();
        values.insert(stage.clone(), json!("Qualified"));
        let operation = commit(
            app,
            Command::UpdateRecord {
                object_id: object.clone(),
                record_id: record.clone(),
                values,
                expected_revision: Some(case.base_record_revision),
            },
        )?;
        app.record_evaluation_outcome(
            &case.id,
            HumanEvaluationOutcomeInput::CommittedOperation {
                operation_id: operation.operation_id,
            },
        )?;
    } else {
        app.record_evaluation_outcome(
            &case.id,
            HumanEvaluationOutcomeInput::NoChange {
                actor: ActorContext::local_user(),
                reason: Some("SYNTHETIC_PRIVATE_REASON".into()),
            },
        )?;
    }
    let _ = index;
    Ok((case, record, shadow_safe))
}

fn create_quality_dataset(
    app: &mut SqliteApplication,
    name: &str,
    cases: Vec<EvaluationCaseId>,
) -> Result<EvaluationDataset, RowvaError> {
    app.create_evaluation_dataset(EvaluationDatasetCreateV1 {
        protocol_version: 1,
        name: name.into(),
        description: Some("synthetic quality fixture".into()),
        selection: EvaluationDatasetSelectionV1::Explicit { case_ids: cases },
        creator: ActorContext::local_user(),
    })
}
fn quality_request(dataset: &EvaluationDataset) -> Result<EvaluationAnalysisRequestV1, RowvaError> {
    Ok(EvaluationAnalysisRequestV1 {
        protocol_version: 1,
        dataset_id: dataset.manifest.dataset_id.clone(),
        actor_id: quality_agent()?.id,
        actor_version: "quality-v1".into(),
        scorer_revision: 1,
        readiness_rubric_revision: 1,
        quality_revision: 1,
        calibration_revision: 1,
    })
}

fn run_quality_fixture(file: QualityFixtureFile) -> Result<QualityFixtureSummary, RowvaError> {
    if file.fixture_protocol_version != 1 || !file.synthetic {
        return Err(RowvaError::validation(
            "unsupported_quality_fixture",
            "quality fixture must be synthetic protocol version 1",
        ));
    }
    let required = [
        "balanced_references",
        "imbalanced_no_change",
        "duplicate_content",
        "repeated_meeting_evidence",
        "repeated_source_meeting_id",
        "missing_candidate_coverage",
        "abstentions",
        "invalid_candidates",
        "well_calibrated_confidence",
        "overconfident_incorrect",
        "underconfident_correct",
        "invalidation_after_dataset",
        "historical_analysis_immutable",
        "new_analysis_excludes_invalidated",
        "metadata_privacy",
        "full_local_sensitive",
    ];
    let names = file
        .scenarios
        .iter()
        .map(|v| v.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    if names.len() != file.scenarios.len() || required.iter().any(|name| !names.contains(name)) {
        return Err(RowvaError::validation(
            "invalid_quality_fixture",
            "quality fixture must declare each required scenario exactly once",
        ));
    }
    let dir = tempfile::tempdir().map_err(|_| RowvaError::Internal {
        code: "fixture_tempdir_failed".into(),
    })?;
    let path = dir.path().join("quality.rowva");
    let mut app = SqliteApplication::create(&path, "Synthetic Quality Fixture")?;
    commit(
        &mut app,
        Command::CreateObject {
            display_name: "Deals".into(),
            key: Some("deals".into()),
        },
    )?;
    let object = app.list_objects()?.remove(0).id;
    commit(
        &mut app,
        Command::CreateField {
            object_id: object.clone(),
            display_name: "Stage".into(),
            key: Some("stage".into()),
            kind: FieldKind::Enum(EnumConfig {
                options: vec![
                    "Discovery".into(),
                    "Qualified".into(),
                    "Negotiation".into(),
                    "SYNTHETIC_PRIVATE_STAGE_VALUE".into(),
                ],
            }),
            required: true,
            unique: false,
        },
    )?;
    commit(
        &mut app,
        Command::CreateField {
            object_id: object.clone(),
            display_name: "Notes".into(),
            key: Some("notes".into()),
            kind: FieldKind::Text(TextConfig::default()),
            required: false,
            unique: false,
        },
    )?;
    let fields = app.list_fields(&object)?;
    let stage = fields
        .iter()
        .find(|f| f.key == "stage")
        .ok_or_else(|| RowvaError::validation("quality_fixture_schema", "stage field missing"))?
        .id
        .clone();
    let notes = fields
        .iter()
        .find(|f| f.key == "notes")
        .ok_or_else(|| RowvaError::validation("quality_fixture_schema", "notes field missing"))?
        .id
        .clone();
    let approvals_before = app.list_approvals(100)?.len();
    let schema_before = app.schema_revision()?;
    let mut shadow_safe = true;
    let mut balanced_cases = Vec::new();
    for index in 0..20 {
        let change = index >= 10;
        let (case, _, safe) = create_quality_case(
            &mut app,
            &object,
            &stage,
            &notes,
            index,
            &format!("balanced_{index}"),
            &format!("balanced_{index}"),
            Some(if change {
                QualityDecision::Change
            } else {
                QualityDecision::NoChange
            }),
            Some(0.9),
            change,
            None,
        )?;
        shadow_safe &= safe;
        balanced_cases.push(case.id);
    }
    let balanced_dataset = create_quality_dataset(&mut app, "balanced", balanced_cases)?;
    let balanced_request = quality_request(&balanced_dataset)?;
    let balanced = app.run_evaluation_analysis(balanced_request.clone())?;
    let balanced_again = app.run_evaluation_analysis(balanced_request)?;
    let mut imbalanced_cases = Vec::new();
    for index in 0..20 {
        let (case, _, safe) = create_quality_case(
            &mut app,
            &object,
            &stage,
            &notes,
            100 + index,
            &format!("imbalanced_{index}"),
            &format!("imbalanced_{index}"),
            Some(QualityDecision::NoChange),
            Some(0.9),
            false,
            None,
        )?;
        shadow_safe &= safe;
        imbalanced_cases.push(case.id);
    }
    let imbalanced_dataset = create_quality_dataset(&mut app, "imbalanced", imbalanced_cases)?;
    let imbalanced = app.run_evaluation_analysis(quality_request(&imbalanced_dataset)?)?;
    let (base, record, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        200,
        "repeated_meeting",
        "repeated_evidence",
        Some(QualityDecision::NoChange),
        Some(0.8),
        false,
        None,
    )?;
    shadow_safe &= safe;
    let (duplicate, _, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        201,
        "repeated_meeting",
        "repeated_evidence",
        Some(QualityDecision::NoChange),
        Some(0.8),
        false,
        Some(record),
    )?;
    shadow_safe &= safe;
    let (missing, _, safe) = create_quality_case(
        &mut app, &object, &stage, &notes, 202, "missing", "missing", None, None, false, None,
    )?;
    shadow_safe &= safe;
    let (abstain, _, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        203,
        "abstain",
        "abstain",
        Some(QualityDecision::Abstain),
        None,
        false,
        None,
    )?;
    shadow_safe &= safe;
    let (invalid, _, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        204,
        "invalid",
        "invalid",
        Some(QualityDecision::Invalid),
        Some(0.9),
        false,
        None,
    )?;
    shadow_safe &= safe;
    let (over, _, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        205,
        "over",
        "over",
        Some(QualityDecision::Change),
        Some(0.99),
        false,
        None,
    )?;
    shadow_safe &= safe;
    let (under, _, safe) = create_quality_case(
        &mut app,
        &object,
        &stage,
        &notes,
        206,
        "under",
        "under",
        Some(QualityDecision::NoChange),
        Some(0.1),
        false,
        None,
    )?;
    shadow_safe &= safe;
    let diagnostic_dataset = create_quality_dataset(
        &mut app,
        "diagnostic",
        vec![
            base.id.clone(),
            duplicate.id.clone(),
            missing.id,
            abstain.id,
            invalid.id,
            over.id.clone(),
            under.id.clone(),
        ],
    )?;
    let diagnostic_request = quality_request(&diagnostic_dataset)?;
    let historical = app.run_evaluation_analysis(diagnostic_request.clone())?;
    let historical_copy = historical.clone();
    let metadata = app.export_evaluation_dataset(
        &diagnostic_dataset.manifest.dataset_id,
        EvaluationExportProfile::Metadata,
    )?;
    let metadata_text = serde_json::to_string(&metadata).map_err(|_| RowvaError::Internal {
        code: "fixture_serialization".into(),
    })?;
    let sentinels = [
        "SYNTHETIC_PRIVATE_MEETING_TEXT",
        "SYNTHETIC_PRIVATE_RECORD_VALUE",
        "SYNTHETIC_PRIVATE_REASON",
        "SYNTHETIC_PRIVATE_STAGE_VALUE",
        "SYNTHETIC_PRIVATE_DISPLAY_NAME",
        "SYNTHETIC_PRIVATE_SESSION",
        "SYNTHETIC_PRIVATE_CLIENT",
    ];
    let metadata_safe = sentinels.iter().all(|v| !metadata_text.contains(v));
    let full = app.export_evaluation_dataset(
        &diagnostic_dataset.manifest.dataset_id,
        EvaluationExportProfile::FullLocal,
    )?;
    let full_text = serde_json::to_string(&full).map_err(|_| RowvaError::Internal {
        code: "fixture_serialization".into(),
    })?;
    let full_sensitive =
        sentinels.iter().all(|v| full_text.contains(v)) && full.payload.contains_sensitive_values;
    let export_digests = sha256_digest(&metadata.payload)? == metadata.export_digest
        && sha256_digest(&full.payload)? == full.export_digest;
    app.invalidate_evaluation_case(EvaluationInvalidationInput {
        protocol_version: 1,
        case_id: base.id.clone(),
        category: EvaluationInvalidationCategory::IncorrectInput,
        reason: "SYNTHETIC_PRIVATE_REASON".into(),
        actor: ActorContext::local_user(),
        related_case_id: None,
    })?;
    let after = app.run_evaluation_analysis(diagnostic_request)?;
    let reloaded_historical = app.get_evaluation_analysis(&historical.id)?;
    let historical_unchanged = reloaded_historical.id == historical_copy.id
        && reloaded_historical.analysis_input_digest == historical_copy.analysis_input_digest
        && reloaded_historical.report_digest == historical_copy.report_digest;
    let warning = |run: &EvaluationAnalysisRun, code: &str| {
        run.report.quality.warnings.iter().any(|v| v.code == code)
    };
    let verdict_confidence = |run: &EvaluationAnalysisRun, case_id: &EvaluationCaseId| {
        run.evidence
            .cases
            .iter()
            .find(|v| &v.case_id == case_id)
            .and_then(|v| v.eligible_candidate.as_ref())
            .and_then(|c| c.confidence)
    };
    let mut actual = HashMap::<&str, bool>::new();
    actual.insert(
        "balanced_references",
        balanced.report.quality.no_change_count == 10
            && balanced.report.quality.stage_change_count == 10
            && balanced.report.metrics.readiness == ReadinessClass::CandidateForHumanReview
            && balanced.id == balanced_again.id,
    );
    actual.insert(
        "imbalanced_no_change",
        imbalanced.report.quality.no_change_count == 20
            && warning(&imbalanced, "reference_label_imbalance"),
    );
    actual.insert(
        "duplicate_content",
        historical
            .report
            .quality
            .duplicate_content_groups
            .iter()
            .any(|v| v.contains(&base.id) && v.contains(&duplicate.id)),
    );
    actual.insert(
        "repeated_meeting_evidence",
        warning(&historical, "repeated_meeting_evidence"),
    );
    actual.insert(
        "repeated_source_meeting_id",
        warning(&historical, "repeated_source_meeting_id"),
    );
    actual.insert(
        "missing_candidate_coverage",
        historical.report.quality.missing_candidate_cases.len() == 1
            && historical.report.quality.eligible_candidate_coverage == Some(6.0 / 7.0),
    );
    actual.insert(
        "abstentions",
        historical.report.quality.abstention_count == 1,
    );
    actual.insert(
        "invalid_candidates",
        historical.report.quality.invalid_candidate_count == 1,
    );
    actual.insert(
        "well_calibrated_confidence",
        balanced.report.calibration.calibrated_sample_count == 20
            && balanced
                .report
                .calibration
                .brier_score
                .is_some_and(|v| (v - 0.01).abs() < 1e-9),
    );
    actual.insert(
        "overconfident_incorrect",
        verdict_confidence(&historical, &over.id) == Some(0.99),
    );
    actual.insert(
        "underconfident_correct",
        verdict_confidence(&historical, &under.id) == Some(0.1),
    );
    actual.insert(
        "invalidation_after_dataset",
        after.report.quality.invalidated_after_dataset_creation == 1
            && after.analysis_input_digest != historical.analysis_input_digest,
    );
    actual.insert("historical_analysis_immutable", historical_unchanged);
    actual.insert(
        "new_analysis_excludes_invalidated",
        after
            .report
            .excluded_cases
            .get(&base.id)
            .map(String::as_str)
            == Some("invalidated"),
    );
    actual.insert("metadata_privacy", metadata_safe && export_digests);
    actual.insert("full_local_sensitive", full_sensitive && export_digests);
    let authority_safe = app.list_approvals(100)?.len() == approvals_before
        && app.schema_revision()? == schema_before;
    let scenarios = file
        .scenarios
        .into_iter()
        .map(|expectation| {
            let computed = *actual.get(expectation.name.as_str()).unwrap_or(&false)
                && shadow_safe
                && authority_safe;
            QualityFixtureResult {
                name: expectation.name,
                expected: expectation.expected,
                actual: computed,
                passed: computed == expectation.expected,
            }
        })
        .collect::<Vec<_>>();
    let passed = scenarios.iter().filter(|v| v.passed).count();
    let failed = scenarios.len() - passed;
    Ok(QualityFixtureSummary {
        fixture_protocol_version: file.fixture_protocol_version,
        scenarios_attempted: scenarios.len(),
        passed,
        failed,
        scenarios,
        privacy_checks: json!({"metadata_sentinels_excluded":metadata_safe,"full_local_sentinels_present":full_sensitive,"export_digests_recomputed":export_digests}),
        integrity_checks: json!({"identical_analysis_idempotent":balanced.id==balanced_again.id,"historical_run_unchanged":historical_unchanged,"invalidation_changed_digest":after.analysis_input_digest!=historical.analysis_input_digest,"shadow_record_mutations":!shadow_safe,"approval_changes":app.list_approvals(100)?.len() as i64-approvals_before as i64,"schema_revision_unchanged":app.schema_revision()?==schema_before}),
    })
}
fn run(cli: Cli) -> Result<(), RowvaError> {
    match cli.command {
        Top::Case {
            command: Case::List { workspace },
        } => output(&open(&workspace)?.list_evaluation_cases()?, cli.json),
        Top::Case {
            command: Case::Invalidate { workspace, input },
        } => {
            let mut app = open(&workspace)?;
            output(&app.invalidate_evaluation_case(read(&input)?)?, cli.json)
        }
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
        Top::Dataset {
            command: DatasetCommand::Create { workspace, input },
        } => {
            let mut app = open(&workspace)?;
            output(&app.create_evaluation_dataset(read(&input)?)?, cli.json)
        }
        Top::Dataset {
            command: DatasetCommand::List { workspace },
        } => output(&open(&workspace)?.list_evaluation_datasets()?, cli.json),
        Top::Dataset {
            command:
                DatasetCommand::Show {
                    workspace,
                    dataset_id,
                },
        } => output(
            &open(&workspace)?
                .get_evaluation_dataset(&EvaluationDatasetId::from_string(dataset_id)?)?,
            cli.json,
        ),
        Top::Dataset {
            command:
                DatasetCommand::Quality {
                    workspace,
                    dataset_id,
                    actor_id,
                    actor_version,
                },
        } => {
            let mut app = open(&workspace)?;
            let dataset_id = EvaluationDatasetId::from_string(dataset_id)?;
            let run = app.run_evaluation_analysis(EvaluationAnalysisRequestV1 {
                protocol_version: 1,
                dataset_id,
                actor_id: ActorId::from_string(actor_id)?,
                actor_version,
                scorer_revision: 1,
                readiness_rubric_revision: 1,
                quality_revision: 1,
                calibration_revision: 1,
            })?;
            output(&run.report.quality, cli.json)
        }
        Top::Dataset {
            command:
                DatasetCommand::Export {
                    workspace,
                    dataset_id,
                    profile,
                    output: destination,
                },
        } => {
            let profile = if profile == ExportProfileArg::Metadata {
                EvaluationExportProfile::Metadata
            } else {
                EvaluationExportProfile::FullLocal
            };
            let export = open(&workspace)?.export_evaluation_dataset(
                &EvaluationDatasetId::from_string(dataset_id)?,
                profile,
            )?;
            if profile == EvaluationExportProfile::FullLocal {
                let path = destination.ok_or_else(|| {
                    RowvaError::validation(
                        "full_local_output_required",
                        "full-local export requires --output and is never printed to stdout",
                    )
                })?;
                eprintln!("warning: full-local RowvAI evaluation export contains sensitive values");
                write_sensitive_export(&path, &export)
            } else if let Some(path) = destination {
                fs::write(
                    path,
                    serde_json::to_vec_pretty(&export).map_err(|_| {
                        RowvaError::validation(
                            "export_serialization_failed",
                            "unable to serialize export",
                        )
                    })?,
                )
                .map_err(|_| {
                    RowvaError::validation("export_write_failed", "unable to write metadata export")
                })
            } else {
                output(&export, cli.json)
            }
        }
        Top::Analysis {
            command: AnalysisCommand::Run { workspace, input },
        } => {
            let mut app = open(&workspace)?;
            output(&app.run_evaluation_analysis(read(&input)?)?, cli.json)
        }
        Top::Analysis {
            command: AnalysisCommand::List { workspace },
        } => output(&open(&workspace)?.list_evaluation_analyses()?, cli.json),
        Top::Analysis {
            command:
                AnalysisCommand::Show {
                    workspace,
                    analysis_id,
                },
        } => output(
            &open(&workspace)?
                .get_evaluation_analysis(&EvaluationAnalysisRunId::from_string(analysis_id)?)?,
            cli.json,
        ),
        Top::Fixture {
            command: Fixture::Run { fixture },
        } => {
            let summary = run_fixture_file(read(&fixture)?)?;
            output(&summary, cli.json)?;
            if summary.failed > 0 {
                return Err(RowvaError::validation(
                    "fixture_expectation_failed",
                    "one or more fixture scenarios failed",
                ));
            }
            Ok(())
        }
        Top::Fixture {
            command: Fixture::QualityRun { fixture },
        } => {
            let summary = run_quality_fixture(read(&fixture)?)?;
            output(&summary, cli.json)?;
            if summary.failed > 0 {
                return Err(RowvaError::validation(
                    "quality_fixture_expectation_failed",
                    "one or more quality fixture scenarios failed",
                ));
            }
            Ok(())
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
