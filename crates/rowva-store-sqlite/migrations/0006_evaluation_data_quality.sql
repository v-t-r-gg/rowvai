CREATE TABLE _rowva_evaluation_invalidations (
 id TEXT PRIMARY KEY, case_id TEXT NOT NULL UNIQUE REFERENCES _rowva_evaluation_cases(id), category TEXT NOT NULL,
 reason TEXT NOT NULL, actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), actor_json TEXT NOT NULL,
 previous_status TEXT NOT NULL, invalidated_at TEXT NOT NULL, case_bundle_digest TEXT NOT NULL,
 related_case_id TEXT REFERENCES _rowva_evaluation_cases(id), protocol_version INTEGER NOT NULL
);
CREATE TABLE _rowva_evaluation_datasets (
 id TEXT PRIMARY KEY, manifest_json TEXT NOT NULL, dataset_digest TEXT NOT NULL UNIQUE,
 workflow TEXT NOT NULL, workflow_version INTEGER NOT NULL, creator_actor_id TEXT NOT NULL REFERENCES _rowva_actors(id), created_at TEXT NOT NULL
);
CREATE TABLE _rowva_evaluation_dataset_cases (
 dataset_id TEXT NOT NULL REFERENCES _rowva_evaluation_datasets(id), case_id TEXT NOT NULL REFERENCES _rowva_evaluation_cases(id),
 position INTEGER NOT NULL, case_bundle_digest TEXT NOT NULL, status_at_snapshot TEXT NOT NULL,
 PRIMARY KEY(dataset_id,case_id), UNIQUE(dataset_id,position)
);
CREATE TABLE _rowva_evaluation_analysis_runs (
 id TEXT PRIMARY KEY, dataset_id TEXT NOT NULL REFERENCES _rowva_evaluation_datasets(id), dataset_digest TEXT NOT NULL,
 actor_id TEXT NOT NULL, actor_version TEXT NOT NULL, scorer_revision INTEGER NOT NULL, readiness_rubric_revision INTEGER NOT NULL,
 quality_revision INTEGER NOT NULL, calibration_revision INTEGER NOT NULL, analysis_input_digest TEXT NOT NULL UNIQUE,
 report_json TEXT NOT NULL, report_digest TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE INDEX _rowva_eval_invalidations_case_time ON _rowva_evaluation_invalidations(case_id,invalidated_at);
CREATE INDEX _rowva_eval_datasets_workflow_time ON _rowva_evaluation_datasets(workflow,workflow_version,created_at);
CREATE INDEX _rowva_eval_analysis_actor ON _rowva_evaluation_analysis_runs(actor_id,actor_version,created_at);
CREATE TRIGGER _rowva_eval_invalidations_no_update BEFORE UPDATE ON _rowva_evaluation_invalidations BEGIN SELECT RAISE(ABORT,'immutable evaluation invalidation'); END;
CREATE TRIGGER _rowva_eval_invalidations_no_delete BEFORE DELETE ON _rowva_evaluation_invalidations BEGIN SELECT RAISE(ABORT,'immutable evaluation invalidation'); END;
CREATE TRIGGER _rowva_eval_datasets_no_update BEFORE UPDATE ON _rowva_evaluation_datasets BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset'); END;
CREATE TRIGGER _rowva_eval_datasets_no_delete BEFORE DELETE ON _rowva_evaluation_datasets BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset'); END;
CREATE TRIGGER _rowva_eval_dataset_cases_no_update BEFORE UPDATE ON _rowva_evaluation_dataset_cases BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset membership'); END;
CREATE TRIGGER _rowva_eval_dataset_cases_no_delete BEFORE DELETE ON _rowva_evaluation_dataset_cases BEGIN SELECT RAISE(ABORT,'immutable evaluation dataset membership'); END;
CREATE TRIGGER _rowva_eval_analysis_no_update BEFORE UPDATE ON _rowva_evaluation_analysis_runs BEGIN SELECT RAISE(ABORT,'immutable evaluation analysis'); END;
CREATE TRIGGER _rowva_eval_analysis_no_delete BEFORE DELETE ON _rowva_evaluation_analysis_runs BEGIN SELECT RAISE(ABORT,'immutable evaluation analysis'); END;
CREATE TRIGGER _rowva_eval_results_no_update BEFORE UPDATE ON _rowva_evaluation_results BEGIN SELECT RAISE(ABORT,'immutable evaluation scorer result'); END;
CREATE TRIGGER _rowva_eval_results_no_delete BEFORE DELETE ON _rowva_evaluation_results BEGIN SELECT RAISE(ABORT,'immutable evaluation scorer result'); END;
