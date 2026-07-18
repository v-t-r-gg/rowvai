use rowva_core::*;
use rowva_store_sqlite::SqliteApplication;
use serde_json::json;
use std::collections::HashMap;
use std::process::Command as ProcessCommand;
#[test]
fn fixture_runs_offline_and_invalid_input_fails() {
    let bin = env!("CARGO_BIN_EXE_rowva-eval");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/shadow/deal_stage_cases.json");
    let out = ProcessCommand::new(bin)
        .args(["--json", "fixture", "run"])
        .arg(root)
        .output()
        .unwrap();
    assert!(out.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(summary["scenario_count"], 10);
    assert_eq!(summary["passed"], 10);
    assert_eq!(summary["failed"], 0);
    assert!(summary["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .all(|scenario| scenario["passed"] == true
            && scenario["replay_deterministic"] == true
            && scenario["mutation_safe"] == true));
    let missing_dir = tempfile::tempdir().unwrap();
    let missing_workspace = missing_dir.path().join("missing.rowva");
    let out = ProcessCommand::new(bin)
        .args([
            "case",
            "show",
            "--workspace",
            missing_workspace.to_str().unwrap(),
            "--case-id",
            "bad",
        ])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn process_case_export_import_outcome_replay_and_report() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("eval.rowva");
    let mut app = SqliteApplication::create(&workspace, "Synthetic Eval").unwrap();
    let actor = ActorContext::local_user();
    app.execute(OperationRequest::new(
        actor.clone(),
        Command::CreateObject {
            display_name: "Deals".into(),
            key: Some("deals".into()),
        },
    ))
    .unwrap();
    let object = app.list_objects().unwrap()[0].id.clone();
    app.execute(OperationRequest::new(
        actor.clone(),
        Command::CreateField {
            object_id: object.clone(),
            display_name: "Stage".into(),
            key: Some("stage".into()),
            kind: FieldKind::Enum(EnumConfig {
                options: vec!["Discovery".into(), "Qualified".into()],
            }),
            required: true,
            unique: false,
        },
    ))
    .unwrap();
    let field = app.list_fields(&object).unwrap()[0].id.clone();
    let mut values = HashMap::new();
    values.insert(field.clone(), json!("Discovery"));
    let created = app
        .execute(OperationRequest::new(
            actor.clone(),
            Command::CreateRecord {
                object_id: object.clone(),
                record_id: None,
                values,
            },
        ))
        .unwrap();
    let record =
        RecordId::from_string(created.result.unwrap()["record_id"].as_str().unwrap()).unwrap();
    drop(app);
    let case_input = EvaluationCaseCreate {
        protocol_version: 1,
        target_object_id: object.clone(),
        target_record_id: record.clone(),
        target_stage_field_id: field.clone(),
        relevant_field_ids: vec![],
        input: DealStageQualificationInputV1 {
            source_meeting_id: Some("synthetic".into()),
            meeting_evidence: json!({"note":"SYNTHETIC_PRIVATE_MEETING_TEXT"}),
            current_stage: json!("Discovery"),
        },
        creator: actor.clone(),
    };
    let case_file = dir.path().join("case.json");
    std::fs::write(&case_file, serde_json::to_vec(&case_input).unwrap()).unwrap();
    let bin = env!("CARGO_BIN_EXE_rowva-eval");
    let call = |args: &[&str]| {
        ProcessCommand::new(bin)
            .arg("--json")
            .args(args)
            .output()
            .unwrap()
    };
    let created = call(&[
        "case",
        "create",
        "--workspace",
        workspace.to_str().unwrap(),
        "--input",
        case_file.to_str().unwrap(),
    ]);
    assert!(created.status.success());
    let case: EvaluationCase = serde_json::from_slice(&created.stdout).unwrap();
    let exported = call(&[
        "case",
        "export",
        "--workspace",
        workspace.to_str().unwrap(),
        "--case-id",
        case.id.as_str(),
    ]);
    assert!(exported.status.success());
    assert!(!String::from_utf8_lossy(&exported.stdout).contains("human_outcome"));
    let candidate_input = CandidateImport {
        protocol_version: 1,
        case_id: case.id.clone(),
        bundle_digest: case.bundle_digest.clone(),
        actor: ActorContext {
            id: ActorId::from_string("act_cli_shadow").unwrap(),
            actor_type: ActorType::Agent,
            display_name: "CLI Shadow".into(),
            capabilities: vec![],
            actor_version: Some("1.0".into()),
            human_principal_id: None,
            client_name: Some("cli-test".into()),
            session_id: None,
        },
        decision: ShadowDecision::NoChange {
            reason: Some("insufficient evidence".into()),
        },
        reason: None,
        confidence: Some(0.6),
    };
    let candidate_file = dir.path().join("candidate.json");
    std::fs::write(
        &candidate_file,
        serde_json::to_vec(&candidate_input).unwrap(),
    )
    .unwrap();
    let submitted = call(&[
        "candidate",
        "submit",
        "--workspace",
        workspace.to_str().unwrap(),
        "--input",
        candidate_file.to_str().unwrap(),
    ]);
    assert!(submitted.status.success());
    let candidate: EvaluationCandidate = serde_json::from_slice(&submitted.stdout).unwrap();
    let outcome = json!({"case_id":case.id,"outcome":{"type":"no_change","actor":actor,"reason":"human reference label"}});
    let outcome_file = dir.path().join("outcome.json");
    std::fs::write(&outcome_file, serde_json::to_vec(&outcome).unwrap()).unwrap();
    assert!(call(&[
        "outcome",
        "record",
        "--workspace",
        workspace.to_str().unwrap(),
        "--input",
        outcome_file.to_str().unwrap()
    ])
    .status
    .success());
    assert!(call(&[
        "replay",
        "--workspace",
        workspace.to_str().unwrap(),
        "--case-id",
        case.id.as_str(),
        "--candidate-id",
        candidate.id.as_str()
    ])
    .status
    .success());
    let report = call(&["report", "--workspace", workspace.to_str().unwrap()]);
    assert!(report.status.success());
    assert!(String::from_utf8_lossy(&report.stdout).contains("insufficient_evidence"));
    let dataset_input = EvaluationDatasetCreateV1 {
        protocol_version: 1,
        name: "CLI dataset".into(),
        description: None,
        selection: EvaluationDatasetSelectionV1::Explicit {
            case_ids: vec![case.id.clone()],
        },
        creator: ActorContext::local_user(),
    };
    let dataset_file = dir.path().join("dataset.json");
    std::fs::write(&dataset_file, serde_json::to_vec(&dataset_input).unwrap()).unwrap();
    let dataset_output = call(&[
        "dataset",
        "create",
        "--workspace",
        workspace.to_str().unwrap(),
        "--input",
        dataset_file.to_str().unwrap(),
    ]);
    assert!(
        dataset_output.status.success(),
        "{}",
        String::from_utf8_lossy(&dataset_output.stderr)
    );
    let dataset: EvaluationDataset = serde_json::from_slice(&dataset_output.stdout).unwrap();
    let analysis_input = EvaluationAnalysisRequestV1 {
        protocol_version: 1,
        dataset_id: dataset.manifest.dataset_id.clone(),
        actor_id: ActorId::from_string("act_cli_shadow").unwrap(),
        actor_version: "1.0".into(),
        scorer_revision: 1,
        readiness_rubric_revision: 1,
        quality_revision: 1,
        calibration_revision: 1,
    };
    let analysis_file = dir.path().join("analysis.json");
    std::fs::write(&analysis_file, serde_json::to_vec(&analysis_input).unwrap()).unwrap();
    assert!(call(&[
        "analysis",
        "run",
        "--workspace",
        workspace.to_str().unwrap(),
        "--input",
        analysis_file.to_str().unwrap()
    ])
    .status
    .success());
    let metadata = call(&[
        "dataset",
        "export",
        "--workspace",
        workspace.to_str().unwrap(),
        "--dataset-id",
        dataset.manifest.dataset_id.as_str(),
        "--profile",
        "metadata",
    ]);
    assert!(metadata.status.success());
    assert!(String::from_utf8_lossy(&metadata.stdout).contains("metadata_only"));
    assert!(!String::from_utf8_lossy(&metadata.stdout).contains("SYNTHETIC_PRIVATE_MEETING_TEXT"));
    let sensitive = dir.path().join("full-local.json");
    assert!(call(&[
        "dataset",
        "export",
        "--workspace",
        workspace.to_str().unwrap(),
        "--dataset-id",
        dataset.manifest.dataset_id.as_str(),
        "--profile",
        "full_local",
        "--output",
        sensitive.to_str().unwrap()
    ])
    .status
    .success());
    let sensitive_text = std::fs::read_to_string(&sensitive).unwrap();
    assert!(sensitive_text.contains("full_local_sensitive"));
    assert!(sensitive_text.contains("SYNTHETIC_PRIVATE_MEETING_TEXT"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&sensitive).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
