CREATE TABLE _rowva_evaluation_cases (
 id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES _rowva_workspace(id), workflow TEXT NOT NULL, workflow_version INTEGER NOT NULL,
 target_object_id TEXT NOT NULL REFERENCES _rowva_objects(id), target_record_id TEXT NOT NULL REFERENCES _rowva_records(id), target_stage_field_id TEXT NOT NULL REFERENCES _rowva_fields(id),
 base_schema_revision INTEGER NOT NULL, base_record_revision INTEGER NOT NULL, current_stage_json TEXT NOT NULL, record_snapshot_json TEXT NOT NULL,
 input_json TEXT NOT NULL, target_field_json TEXT NOT NULL, evidence_digest TEXT NOT NULL, bundle_digest TEXT NOT NULL UNIQUE,
 creator_actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), creator_json TEXT NOT NULL, created_at TEXT NOT NULL, status TEXT NOT NULL,
 invalidated_at TEXT, invalidation_reason TEXT
);
CREATE TABLE _rowva_evaluation_candidates (
 id TEXT PRIMARY KEY, case_id TEXT NOT NULL REFERENCES _rowva_evaluation_cases(id), workflow_version INTEGER NOT NULL, bundle_digest TEXT NOT NULL,
 actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), actor_json TEXT NOT NULL, decision_json TEXT NOT NULL, reason TEXT, confidence REAL,
 fingerprint TEXT NOT NULL, submitted_at TEXT NOT NULL, validation_status TEXT NOT NULL, validation_error_json TEXT,
 eligible_for_metrics INTEGER NOT NULL, normalized_changes_json TEXT NOT NULL, UNIQUE(case_id,actor_id,fingerprint)
);
CREATE TABLE _rowva_evaluation_human_outcomes (case_id TEXT PRIMARY KEY REFERENCES _rowva_evaluation_cases(id), outcome_json TEXT NOT NULL, normalized_stage_json TEXT NOT NULL, recorded_at TEXT NOT NULL);
CREATE TABLE _rowva_evaluation_results (
 id TEXT PRIMARY KEY, case_id TEXT NOT NULL REFERENCES _rowva_evaluation_cases(id), candidate_id TEXT NOT NULL REFERENCES _rowva_evaluation_candidates(id),
 scorer_revision INTEGER NOT NULL, candidate_stage_json TEXT, human_stage_json TEXT NOT NULL, verdict TEXT NOT NULL, eligible INTEGER NOT NULL,
 scored_at TEXT NOT NULL, UNIQUE(candidate_id,scorer_revision)
);
CREATE INDEX _rowva_eval_cases_workflow_status ON _rowva_evaluation_cases(workflow,workflow_version,status,created_at);
CREATE INDEX _rowva_eval_candidates_actor ON _rowva_evaluation_candidates(actor_id,submitted_at);
CREATE INDEX _rowva_eval_candidates_case ON _rowva_evaluation_candidates(case_id,submitted_at);
CREATE INDEX _rowva_eval_results_case ON _rowva_evaluation_results(case_id,scored_at);
