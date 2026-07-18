CREATE TABLE _rowva_evaluation_invalidations (
 id TEXT PRIMARY KEY, case_id TEXT NOT NULL UNIQUE REFERENCES _rowva_evaluation_cases(id), category TEXT NOT NULL,
 reason TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), actor_json TEXT NOT NULL,
 previous_status TEXT NOT NULL, invalidated_at TEXT NOT NULL, case_bundle_digest TEXT NOT NULL,
 related_case_id TEXT REFERENCES _rowva_evaluation_cases(id), protocol_version INTEGER NOT NULL
);
CREATE TABLE _rowva_evaluation_datasets (
 id TEXT PRIMARY KEY, manifest_json TEXT NOT NULL, dataset_digest TEXT NOT NULL UNIQUE,
 workflow TEXT NOT NULL, workflow_version INTEGER NOT NULL, creator_actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), created_at TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('building','sealed'))
);
CREATE TABLE _rowva_evaluation_dataset_cases (
 dataset_id TEXT NOT NULL REFERENCES _rowva_evaluation_datasets(id), case_id TEXT NOT NULL REFERENCES _rowva_evaluation_cases(id),
 position INTEGER NOT NULL, case_bundle_digest TEXT NOT NULL, status_at_snapshot TEXT NOT NULL,
 case_created_at TEXT NOT NULL, human_outcome_present INTEGER NOT NULL, scorer_revisions_json TEXT NOT NULL,
 PRIMARY KEY(dataset_id,case_id), UNIQUE(dataset_id,position)
);
CREATE TABLE _rowva_evaluation_analysis_runs (
 id TEXT PRIMARY KEY, dataset_id TEXT NOT NULL REFERENCES _rowva_evaluation_datasets(id), dataset_digest TEXT NOT NULL,
 actor_id TEXT NOT NULL, actor_version TEXT NOT NULL, scorer_revision INTEGER NOT NULL, readiness_rubric_revision INTEGER NOT NULL,
 quality_revision INTEGER NOT NULL, calibration_revision INTEGER NOT NULL, analysis_input_digest TEXT NOT NULL UNIQUE,
 evidence_json TEXT NOT NULL, report_json TEXT NOT NULL, report_digest TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE INDEX _rowva_eval_invalidations_case_time ON _rowva_evaluation_invalidations(case_id,invalidated_at);
CREATE INDEX _rowva_eval_datasets_workflow_time ON _rowva_evaluation_datasets(workflow,workflow_version,created_at);
CREATE INDEX _rowva_eval_analysis_actor ON _rowva_evaluation_analysis_runs(actor_id,actor_version,created_at);
CREATE TRIGGER _rowva_eval_invalidations_no_update BEFORE UPDATE ON _rowva_evaluation_invalidations BEGIN SELECT RAISE(ABORT,'immutable evaluation invalidation'); END;
CREATE TRIGGER _rowva_eval_invalidations_no_delete BEFORE DELETE ON _rowva_evaluation_invalidations BEGIN SELECT RAISE(ABORT,'immutable evaluation invalidation'); END;
CREATE TRIGGER _rowva_eval_datasets_no_update BEFORE UPDATE ON _rowva_evaluation_datasets
WHEN NOT (OLD.status='building' AND NEW.status='sealed' AND OLD.id=NEW.id AND OLD.manifest_json=NEW.manifest_json AND OLD.dataset_digest=NEW.dataset_digest AND OLD.workflow=NEW.workflow AND OLD.workflow_version=NEW.workflow_version AND OLD.creator_actor_id=NEW.creator_actor_id AND OLD.created_at=NEW.created_at)
BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset'); END;
CREATE TRIGGER _rowva_eval_datasets_no_delete BEFORE DELETE ON _rowva_evaluation_datasets BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset'); END;
CREATE TRIGGER _rowva_eval_dataset_cases_sealed_insert BEFORE INSERT ON _rowva_evaluation_dataset_cases WHEN (SELECT status FROM _rowva_evaluation_datasets WHERE id=NEW.dataset_id)!='building' BEGIN SELECT RAISE(ABORT,'sealed evaluation dataset membership'); END;
CREATE TRIGGER _rowva_eval_dataset_cases_no_update BEFORE UPDATE ON _rowva_evaluation_dataset_cases BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset membership'); END;
CREATE TRIGGER _rowva_eval_dataset_cases_no_delete BEFORE DELETE ON _rowva_evaluation_dataset_cases BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset membership'); END;
CREATE TRIGGER _rowva_eval_analysis_no_update BEFORE UPDATE ON _rowva_evaluation_analysis_runs BEGIN SELECT RAISE(ABORT,'immutable evaluation analysis'); END;
CREATE TRIGGER _rowva_eval_analysis_no_delete BEFORE DELETE ON _rowva_evaluation_analysis_runs BEGIN SELECT RAISE(ABORT,'immutable evaluation analysis'); END;
CREATE TRIGGER _rowva_eval_results_no_update BEFORE UPDATE ON _rowva_evaluation_results BEGIN SELECT RAISE(ABORT,'immutable evaluation scorer result'); END;
CREATE TRIGGER _rowva_eval_results_no_delete BEFORE DELETE ON _rowva_evaluation_results BEGIN SELECT RAISE(ABORT,'immutable evaluation scorer result'); END;
CREATE TRIGGER _rowva_eval_candidates_no_update BEFORE UPDATE ON _rowva_evaluation_candidates BEGIN SELECT RAISE(ABORT,'immutable evaluation candidate'); END;
CREATE TRIGGER _rowva_eval_candidates_no_delete BEFORE DELETE ON _rowva_evaluation_candidates BEGIN SELECT RAISE(ABORT,'immutable evaluation candidate'); END;
CREATE TRIGGER _rowva_eval_outcomes_no_update BEFORE UPDATE ON _rowva_evaluation_human_outcomes BEGIN SELECT RAISE(ABORT,'immutable evaluation human outcome'); END;
CREATE TRIGGER _rowva_eval_outcomes_no_delete BEFORE DELETE ON _rowva_evaluation_human_outcomes BEGIN SELECT RAISE(ABORT,'immutable evaluation human outcome'); END;
CREATE TRIGGER _rowva_eval_cases_frozen_no_update BEFORE UPDATE ON _rowva_evaluation_cases
WHEN OLD.workspace_id IS NOT NEW.workspace_id OR OLD.workflow IS NOT NEW.workflow OR OLD.workflow_version IS NOT NEW.workflow_version OR OLD.target_object_id IS NOT NEW.target_object_id OR OLD.target_record_id IS NOT NEW.target_record_id OR OLD.target_stage_field_id IS NOT NEW.target_stage_field_id OR OLD.base_schema_revision IS NOT NEW.base_schema_revision OR OLD.base_record_revision IS NOT NEW.base_record_revision OR OLD.current_stage_json IS NOT NEW.current_stage_json OR OLD.record_snapshot_json IS NOT NEW.record_snapshot_json OR OLD.input_json IS NOT NEW.input_json OR OLD.target_field_json IS NOT NEW.target_field_json OR OLD.evidence_digest IS NOT NEW.evidence_digest OR OLD.bundle_digest IS NOT NEW.bundle_digest OR OLD.creator_actor_id IS NOT NEW.creator_actor_id OR OLD.creator_json IS NOT NEW.creator_json OR OLD.created_at IS NOT NEW.created_at
BEGIN SELECT RAISE(ABORT,'immutable frozen evaluation case'); END;
CREATE TRIGGER _rowva_eval_cases_status_transition BEFORE UPDATE OF status ON _rowva_evaluation_cases
WHEN NOT ((OLD.status='collecting_candidates' AND NEW.status IN ('outcome_recorded','invalidated')) OR (OLD.status='outcome_recorded' AND NEW.status='scored') OR (OLD.status='scored' AND NEW.status='invalidated'))
BEGIN SELECT RAISE(ABORT,'invalid evaluation case transition'); END;
