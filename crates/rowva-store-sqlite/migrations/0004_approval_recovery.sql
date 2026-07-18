ALTER TABLE _rowva_operations ADD COLUMN expires_at TEXT;
ALTER TABLE _rowva_operations ADD COLUMN policy_revision INTEGER NOT NULL DEFAULT 1;
ALTER TABLE _rowva_operations ADD COLUMN policy_reason_codes_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE _rowva_operations ADD COLUMN supersedes_operation_id TEXT REFERENCES _rowva_operations(id);
ALTER TABLE _rowva_operations ADD COLUMN superseded_by_operation_id TEXT REFERENCES _rowva_operations(id);
ALTER TABLE _rowva_operations ADD COLUMN reverts_operation_id TEXT REFERENCES _rowva_operations(id);
ALTER TABLE _rowva_operations ADD COLUMN reverted_by_operation_id TEXT REFERENCES _rowva_operations(id);

CREATE TABLE _rowva_approval_requests (
  id TEXT PRIMARY KEY,
  operation_id TEXT NOT NULL UNIQUE REFERENCES _rowva_operations(id),
  workspace_id TEXT NOT NULL REFERENCES _rowva_workspace(id),
  proposal_fingerprint TEXT NOT NULL,
  status TEXT NOT NULL,
  required_capability TEXT NOT NULL,
  policy_decision TEXT NOT NULL,
  policy_reason_codes_json TEXT NOT NULL,
  policy_revision INTEGER NOT NULL,
  requested_actor_id TEXT NOT NULL REFERENCES _rowva_actors(id),
  requested_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  decided_at TEXT,
  executed_at TEXT,
  execution_error_json TEXT
);

CREATE TABLE _rowva_approval_decisions (
  id TEXT PRIMARY KEY,
  approval_request_id TEXT NOT NULL REFERENCES _rowva_approval_requests(id),
  operation_id TEXT NOT NULL REFERENCES _rowva_operations(id),
  proposal_fingerprint TEXT NOT NULL,
  decided_by_actor_id TEXT NOT NULL REFERENCES _rowva_actors(id),
  decision TEXT NOT NULL,
  reason TEXT,
  decided_at TEXT NOT NULL
);

CREATE UNIQUE INDEX _rowva_approval_one_decision ON _rowva_approval_decisions(approval_request_id);
CREATE INDEX _rowva_approval_inbox ON _rowva_approval_requests(workspace_id,status,requested_at DESC);
CREATE INDEX _rowva_approval_expiration ON _rowva_approval_requests(status,expires_at);
CREATE INDEX _rowva_operations_status_created ON _rowva_operations(status,created_at DESC);
CREATE INDEX _rowva_operations_supersedes ON _rowva_operations(supersedes_operation_id);
CREATE INDEX _rowva_operations_reverts ON _rowva_operations(reverts_operation_id);
