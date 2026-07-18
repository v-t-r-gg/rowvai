//! SQLite-backed Rowva application service.

use chrono::Utc;
use rowva_core::*;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path, sync::Arc};

const MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        1,
        "foundation-v1",
        include_str!("../migrations/0001_foundation.sql"),
    ),
    (
        2,
        "record-values-v1",
        include_str!("../migrations/0002_record_values.sql"),
    ),
    (
        3,
        "durable-previews-v1",
        include_str!("../migrations/0003_durable_previews.sql"),
    ),
    (
        4,
        "approval-recovery-v1",
        include_str!("../migrations/0004_approval_recovery.sql"),
    ),
];

pub struct SqliteApplication {
    conn: Connection,
    workspace_id: WorkspaceId,
    policy: Arc<dyn Policy>,
}

impl SqliteApplication {
    pub fn create(path: &Path, name: &str) -> Result<Self, RowvaError> {
        if name.trim().is_empty() {
            return Err(RowvaError::validation(
                "empty_workspace_name",
                "workspace name is required",
            ));
        }
        if path.exists() {
            return Err(RowvaError::Conflict {
                code: "workspace_already_exists".into(),
                message: "refusing to overwrite an existing workspace file".into(),
                details: None,
            });
        }
        let conn = Self::connection(path)?;
        let mut app = Self {
            conn,
            workspace_id: WorkspaceId::new(),
            policy: Arc::new(CapabilityPolicy),
        };
        app.migrate()?;
        let now = Utc::now().to_rfc3339();
        app.conn.execute("INSERT INTO _rowva_workspace(id, name, schema_revision, created_at, updated_at) VALUES(?1,?2,0,?3,?3)", params![app.workspace_id.as_str(), name.trim(), now]).map_err(storage)?;
        Ok(app)
    }

    pub fn open(path: &Path) -> Result<Self, RowvaError> {
        let conn = Self::connection(path)?;
        let legacy: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_rowva_tables') AND NOT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='_rowva_migrations')",
                [],
                |row| row.get(0),
            )
            .map_err(storage)?;
        if legacy {
            return Err(RowvaError::Validation {
                code: "legacy_workspace_requires_export".into(),
                message: "unversioned prototype workspaces must be exported with the 0.1 build and recreated".into(),
                details: None,
            });
        }
        let mut app = Self {
            conn,
            workspace_id: WorkspaceId::new(),
            policy: Arc::new(CapabilityPolicy),
        };
        app.migrate()?;
        let id: Option<String> = app
            .conn
            .query_row("SELECT id FROM _rowva_workspace LIMIT 1", [], |r| r.get(0))
            .optional()
            .map_err(storage)?;
        let id = id.ok_or_else(|| {
            RowvaError::validation(
                "not_rowva_workspace",
                "file has no versioned Rowva workspace metadata",
            )
        })?;
        app.workspace_id = WorkspaceId::from_string(id)?;
        Ok(app)
    }

    pub fn with_policy(mut self, policy: Arc<dyn Policy>) -> Self {
        self.policy = policy;
        self
    }
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }
    pub fn workspace_name(&self) -> Result<String, RowvaError> {
        self.conn
            .query_row(
                "SELECT name FROM _rowva_workspace WHERE id=?1",
                [self.workspace_id.as_str()],
                |r| r.get(0),
            )
            .map_err(storage)
    }
    pub fn schema_revision(&self) -> Result<SchemaRevision, RowvaError> {
        self.conn
            .query_row(
                "SELECT schema_revision FROM _rowva_workspace WHERE id=?1",
                [self.workspace_id.as_str()],
                |r| r.get::<_, i64>(0).map(SchemaRevision),
            )
            .map_err(storage)
    }
    pub fn migration_versions(&self) -> Result<Vec<i64>, RowvaError> {
        let mut s = self
            .conn
            .prepare("SELECT version FROM _rowva_migrations ORDER BY version")
            .map_err(storage)?;
        let result = s
            .query_map([], |r| r.get(0))
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage);
        result
    }

    fn connection(path: &Path) -> Result<Connection, RowvaError> {
        let conn = Connection::open(path).map_err(storage)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(storage)?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
            .map_err(storage)?;
        Ok(conn)
    }

    fn migrate(&mut self) -> Result<(), RowvaError> {
        self.conn.execute_batch("CREATE TABLE IF NOT EXISTS _rowva_migrations(version INTEGER PRIMARY KEY, application_version TEXT NOT NULL, applied_at TEXT NOT NULL, identifier TEXT NOT NULL);").map_err(storage)?;
        for (version, identifier, sql) in MIGRATIONS {
            let exists: bool = self
                .conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM _rowva_migrations WHERE version=?1)",
                    [version],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            if !exists {
                let tx = self.conn.transaction().map_err(storage)?;
                tx.execute_batch(sql).map_err(storage)?;
                tx.execute("INSERT INTO _rowva_migrations(version,application_version,applied_at,identifier) VALUES(?1,?2,?3,?4)", params![version, env!("CARGO_PKG_VERSION"), Utc::now().to_rfc3339(), identifier]).map_err(storage)?;
                tx.commit().map_err(storage)?;
            }
        }
        Ok(())
    }

    fn command_kind(command: &Command) -> &'static str {
        match command {
            Command::CreateObject { .. } => "create_object",
            Command::CreateField { .. } => "create_field",
            Command::CreateRecord { .. } => "create_record",
            Command::UpdateRecord { .. } => "update_record",
            Command::DeleteRecord { .. } => "delete_record",
        }
    }
    fn slug(value: &str) -> String {
        value
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .trim_matches('_')
            .to_string()
    }
    fn record_failure(
        &self,
        op: &OperationId,
        request: &OperationRequest,
        error: &RowvaError,
        base: SchemaRevision,
    ) {
        let _=self.conn.execute("INSERT OR REPLACE INTO _rowva_operations(id,workspace_id,actor_id,actor_type,actor_display_name,kind,mode,status,reason,idempotency_key,request_json,error_json,created_at,base_schema_revision) VALUES(?1,?2,?3,?4,?5,?6,?7,'failed',?8,?9,?10,?11,?12,?13)", params![op.as_str(),self.workspace_id.as_str(),request.actor.id.as_str(),actor_type(request.actor.actor_type),request.actor.display_name,Self::command_kind(&request.command),mode(request.mode),request.reason,request.idempotency_key,serde_json::to_string(request).ok(),serde_json::to_string(error).ok(),Utc::now().to_rfc3339(),base.0]);
    }

    fn execute_transaction(
        &mut self,
        request: &OperationRequest,
        op: &OperationId,
        base: SchemaRevision,
    ) -> Result<OperationResponse, RowvaError> {
        let tx = self.conn.transaction().map_err(storage)?;
        let now = Utc::now().to_rfc3339();
        tx.execute("INSERT OR REPLACE INTO _rowva_actors(id,actor_type,display_name,created_at,updated_at) VALUES(?1,?2,?3,COALESCE((SELECT created_at FROM _rowva_actors WHERE id=?1),?4),?4)", params![request.actor.id.as_str(),actor_type(request.actor.actor_type),request.actor.display_name,now]).map_err(storage)?;
        let (changes, result, schema) =
            apply_command(&tx, &self.workspace_id, &request.command, base)?;
        let status = if request.mode == OperationMode::Preview {
            OperationStatus::Proposed
        } else {
            OperationStatus::Committed
        };
        if request.mode == OperationMode::Preview {
            tx.rollback().map_err(storage)?;
            return Ok(OperationResponse {
                protocol_version: 1,
                operation_id: op.clone(),
                status,
                warnings: vec![],
                changes,
                result: Some(result),
                base_schema_revision: base,
                resulting_schema_revision: schema,
                replayed: false,
                undo_token: None,
            });
        }
        tx.execute("INSERT INTO _rowva_operations(id,workspace_id,actor_id,actor_type,actor_display_name,kind,mode,status,reason,idempotency_key,request_json,result_json,created_at,committed_at,base_schema_revision,resulting_schema_revision) VALUES(?1,?2,?3,?4,?5,?6,'commit','committed',?7,?8,?9,?10,?11,?11,?12,?13)", params![op.as_str(),self.workspace_id.as_str(),request.actor.id.as_str(),actor_type(request.actor.actor_type),request.actor.display_name,Self::command_kind(&request.command),request.reason,request.idempotency_key,serde_json::to_string(request).map_err(internal)?,serde_json::to_string(&result).map_err(internal)?,now,base.0,schema.0]).map_err(storage)?;
        for c in &changes {
            tx.execute("INSERT INTO _rowva_operation_changes(operation_id,object_id,record_id,field_id,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)", params![op.as_str(),c.object_id.as_str(),c.record_id.as_ref().map(RecordId::as_str),c.field_id.as_ref().map(FieldId::as_str),c.before.as_ref().map(Value::to_string),c.after.as_ref().map(Value::to_string)]).map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        Ok(OperationResponse {
            protocol_version: 1,
            operation_id: op.clone(),
            status,
            warnings: vec![],
            changes,
            result: Some(result),
            base_schema_revision: base,
            resulting_schema_revision: schema,
            replayed: false,
            undo_token: None,
        })
    }

    fn mark_conflict<T>(
        &self,
        operation_id: &OperationId,
        code: &str,
        message: &str,
    ) -> Result<T, RowvaError> {
        let error = RowvaError::Conflict {
            code: code.into(),
            message: message.into(),
            details: Some(json!({"recovery":{"action":"preview_again"}})),
        };
        self.conn.execute("UPDATE _rowva_operations SET status='conflicted',error_json=?1 WHERE id=?2 AND status='proposed'",params![serde_json::to_string(&error).map_err(internal)?,operation_id.as_str()]).map_err(storage)?;
        Err(error)
    }

    fn expire_proposals(&mut self) -> Result<(), RowvaError> {
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction().map_err(storage)?;
        tx.execute("UPDATE _rowva_approval_requests SET status='expired' WHERE status='pending' AND expires_at<=?1",[&now]).map_err(storage)?;
        tx.execute("UPDATE _rowva_operations SET status='expired' WHERE status='awaiting_approval' AND expires_at<=?1",[&now]).map_err(storage)?;
        tx.commit().map_err(storage)
    }

    fn hydrate_links(&self, operation: &mut OperationRecord) -> Result<(), RowvaError> {
        type OperationLinks = (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let row:OperationLinks=self.conn.query_row("SELECT expires_at,supersedes_operation_id,superseded_by_operation_id,reverts_operation_id,reverted_by_operation_id FROM _rowva_operations WHERE id=?1",[operation.id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).map_err(storage)?;
        operation.expires_at = row.0.map(parse_time);
        operation.supersedes_operation_id = row.1.map(OperationId::from_string).transpose()?;
        operation.superseded_by_operation_id = row.2.map(OperationId::from_string).transpose()?;
        operation.reverts_operation_id = row.3.map(OperationId::from_string).transpose()?;
        operation.reverted_by_operation_id = row.4.map(OperationId::from_string).transpose()?;
        Ok(())
    }

    fn load_approval(&self, id: &ApprovalRequestId) -> Result<ApprovalRequest, RowvaError> {
        type ApprovalRow = (
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            i64,
            String,
            String,
            Option<String>,
            Option<String>,
        );
        let row:ApprovalRow=self.conn.query_row("SELECT operation_id,workspace_id,proposal_fingerprint,status,required_capability,policy_decision,policy_reason_codes_json,policy_revision,requested_at,expires_at,decided_at,executed_at FROM _rowva_approval_requests WHERE id=?1",[id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?))).optional().map_err(storage)?.ok_or_else(||RowvaError::not_found("approval",id.as_str()))?;
        let operation_id = OperationId::from_string(row.0)?;
        let mut operation = self.get_operation(&operation_id)?;
        let requested_by: ActorContext =
            serde_json::from_value(operation.request.get("actor").cloned().ok_or_else(|| {
                RowvaError::Internal {
                    code: "missing_request_actor".into(),
                }
            })?)
            .map_err(internal)?;
        let decision=self.conn.query_row("SELECT d.id,d.proposal_fingerprint,d.decision,d.reason,d.decided_at,a.id,a.actor_type,a.display_name,a.actor_version,a.human_principal_id,a.client_name,a.session_id FROM _rowva_approval_decisions d JOIN _rowva_actors a ON a.id=d.decided_by_actor_id WHERE d.approval_request_id=?1",[id.as_str()],|r|{
            Ok(ApprovalDecision{id:ApprovalDecisionId::from_string(r.get::<_,String>(0)?).map_err(sql_conversion)?,approval_request_id:id.clone(),operation_id:operation_id.clone(),proposal_fingerprint:r.get(1)?,decided_by:ActorContext{id:ActorId::from_string(r.get::<_,String>(5)?).map_err(sql_conversion)?,actor_type:parse_actor(&r.get::<_,String>(6)?),display_name:r.get(7)?,capabilities:vec![],actor_version:r.get(8)?,human_principal_id:r.get(9)?,client_name:r.get(10)?,session_id:r.get(11)?},decision:if r.get::<_,String>(2)?=="approve"{ApprovalDecisionKind::Approve}else{ApprovalDecisionKind::Reject},reason:r.get(3)?,decided_at:parse_time(r.get(4)?),})
        }).optional().map_err(storage)?;
        operation.changes = load_changes(&self.conn, &operation_id)?;
        Ok(ApprovalRequest {
            id: id.clone(),
            operation_id,
            workspace_id: WorkspaceId::from_string(row.1)?,
            proposal_fingerprint: row.2,
            status: parse_approval_status(&row.3),
            requested_by,
            required_approver_capability: parse_capability(&row.4),
            policy_decision: if row.5 == "require_approval" {
                PolicyDecision::RequireApproval
            } else {
                PolicyDecision::Allow
            },
            policy_reason_codes: serde_json::from_str(&row.6).unwrap_or_default(),
            policy_revision: row.7,
            requested_at: parse_time(row.8),
            expires_at: parse_time(row.9),
            decided_at: row.10.map(parse_time),
            executed_at: row.11.map(parse_time),
            operation,
            decision,
        })
    }
}

impl RowvaApplication for SqliteApplication {
    fn execute(&mut self, request: OperationRequest) -> Result<OperationResponse, RowvaError> {
        if request.protocol_version != 1 {
            return Err(RowvaError::validation(
                "unsupported_protocol_version",
                "only protocol version 1 is supported",
            ));
        }
        self.policy.authorize(
            &request.actor,
            request.command.required_capability(),
            request.mode,
        )?;
        let request_json = serde_json::to_string(&request).map_err(internal)?;
        if request.mode == OperationMode::Commit {
            if let Some(key) = &request.idempotency_key {
                let previous:Option<(String,String,String)>=self.conn.query_row("SELECT id,request_json,result_json FROM _rowva_operations WHERE actor_id=?1 AND idempotency_key=?2 AND status='committed'",params![request.actor.id.as_str(),key],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(storage)?;
                if let Some((id, old, result)) = previous {
                    if old != request_json {
                        return Err(RowvaError::Conflict {
                            code: "idempotency_key_reused".into(),
                            message: "idempotency key was already used for a different request"
                                .into(),
                            details: None,
                        });
                    }
                    let result: OperationResponse = serde_json::from_str::<Value>(&result)
                        .map(|v| OperationResponse {
                            protocol_version: 1,
                            operation_id: OperationId::from_string(id).unwrap_or_default(),
                            status: OperationStatus::Committed,
                            warnings: vec![],
                            changes: vec![],
                            result: Some(v),
                            base_schema_revision: self
                                .schema_revision()
                                .unwrap_or(SchemaRevision(0)),
                            resulting_schema_revision: self
                                .schema_revision()
                                .unwrap_or(SchemaRevision(0)),
                            replayed: true,
                            undo_token: None,
                        })
                        .map_err(internal)?;
                    return Ok(result);
                }
            }
        }
        let op = OperationId::new();
        let base = self.schema_revision()?;
        match self.execute_transaction(&request, &op, base) {
            Ok(v) => Ok(v),
            Err(e) => {
                if request.mode == OperationMode::Commit {
                    self.record_failure(&op, &request, &e, base);
                }
                Err(e)
            }
        }
    }
    fn list_objects(&self) -> Result<Vec<ObjectDefinition>, RowvaError> {
        let mut s=self.conn.prepare("SELECT id,display_name,key,created_at,updated_at,schema_revision FROM _rowva_objects ORDER BY display_name").map_err(storage)?;
        let result = s
            .query_map([], object_row)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage);
        result
    }
    fn list_fields(&self, object_id: &ObjectId) -> Result<Vec<FieldDefinition>, RowvaError> {
        let mut s=self.conn.prepare("SELECT id,object_id,display_name,key,kind_json,required,unique_flag,created_at,updated_at,schema_revision FROM _rowva_fields WHERE object_id=?1 ORDER BY position,id").map_err(storage)?;
        let result = s
            .query_map([object_id.as_str()], field_row)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage);
        result
    }
    fn list_records(&self, object_id: &ObjectId) -> Result<Vec<Record>, RowvaError> {
        let mut s=self.conn.prepare("SELECT id,object_id,revision,created_at,updated_at FROM _rowva_records WHERE object_id=?1 ORDER BY created_at,id").map_err(storage)?;
        let headers = s
            .query_map([object_id.as_str()], record_header)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        headers
            .into_iter()
            .map(|mut r| {
                r.values = load_values(&self.conn, &r.record_id)?;
                Ok(r)
            })
            .collect()
    }
    fn get_record(&self, object_id: &ObjectId, record_id: &RecordId) -> Result<Record, RowvaError> {
        let mut r=self.conn.query_row("SELECT id,object_id,revision,created_at,updated_at FROM _rowva_records WHERE id=?1 AND object_id=?2",params![record_id.as_str(),object_id.as_str()],record_header).optional().map_err(storage)?.ok_or_else(||RowvaError::not_found("record",record_id.as_str()))?;
        r.values = load_values(&self.conn, record_id)?;
        Ok(r)
    }
    fn get_operation(&self, operation_id: &OperationId) -> Result<OperationRecord, RowvaError> {
        let mut operation=self.conn.query_row("SELECT id,actor_id,actor_type,actor_display_name,kind,mode,status,reason,idempotency_key,request_json,result_json,error_json,created_at,committed_at,base_schema_revision,resulting_schema_revision,correlation_json,policy_decision,preview_fingerprint FROM _rowva_operations WHERE id=?1",[operation_id.as_str()],operation_row).optional().map_err(storage)?.ok_or_else(||RowvaError::not_found("operation",operation_id.as_str()))?;
        operation.changes = load_changes(&self.conn, operation_id)?;
        self.hydrate_links(&mut operation)?;
        Ok(operation)
    }
    fn list_operations(&self, limit: usize) -> Result<Vec<OperationRecord>, RowvaError> {
        let mut s=self.conn.prepare("SELECT id,actor_id,actor_type,actor_display_name,kind,mode,status,reason,idempotency_key,request_json,result_json,error_json,created_at,committed_at,base_schema_revision,resulting_schema_revision,correlation_json,policy_decision,preview_fingerprint FROM _rowva_operations ORDER BY created_at DESC,id DESC LIMIT ?1").map_err(storage)?;
        let result = s
            .query_map([limit as i64], operation_row)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage);
        let mut operations = result?;
        for operation in &mut operations {
            operation.changes = load_changes(&self.conn, &operation.id)?;
            self.hydrate_links(operation)?;
        }
        Ok(operations)
    }

    fn preview_operation(
        &mut self,
        mut request: OperationRequest,
    ) -> Result<OperationPreview, RowvaError> {
        request.mode = OperationMode::Preview;
        if request.reason.as_ref().is_some_and(|v| v.len() > 2048) {
            return Err(RowvaError::validation(
                "reason_too_long",
                "operation reason exceeds 2048 bytes",
            ));
        }
        validate_correlation(&request.correlation)?;
        self.policy.authorize(
            &request.actor,
            request.command.required_capability(),
            OperationMode::Preview,
        )?;
        if let Some(key) = &request.idempotency_key {
            let prior:Option<(String,String)>=self.conn.query_row("SELECT request_json,result_json FROM _rowva_operations WHERE actor_id=?1 AND idempotency_key=?2",params![request.actor.id.as_str(),key],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(storage)?;
            if let Some((stored, result)) = prior {
                let stored_request: OperationRequest =
                    serde_json::from_str(&stored).map_err(internal)?;
                if let (
                    Command::CreateRecord {
                        record_id: incoming,
                        ..
                    },
                    Command::CreateRecord {
                        record_id: stored, ..
                    },
                ) = (&mut request.command, &stored_request.command)
                {
                    if incoming.is_none() {
                        *incoming = stored.clone();
                    }
                }
                let canonical = canonical_json(&serde_json::to_value(&request).map_err(internal)?);
                if canonical_json(&serde_json::from_str(&stored).map_err(internal)?) != canonical {
                    return Err(RowvaError::Conflict {
                        code: "idempotency_key_reused".into(),
                        message: "idempotency key was used for a different preview".into(),
                        details: None,
                    });
                }
                let mut preview: OperationPreview =
                    serde_json::from_str(&result).map_err(internal)?;
                preview.replayed = true;
                return Ok(preview);
            }
        }
        if let Command::CreateRecord { record_id, .. } = &mut request.command {
            if record_id.is_none() {
                *record_id = Some(RecordId::new());
            }
        }
        let canonical = canonical_json(&serde_json::to_value(&request).map_err(internal)?);
        let op = OperationId::new();
        let base = self.schema_revision()?;
        let created_at = Utc::now();
        let tx = self.conn.transaction().map_err(storage)?;
        let (changes, _result, resulting) =
            apply_command(&tx, &self.workspace_id, &request.command, base)?;
        tx.rollback().map_err(storage)?;
        let witnesses = state_witnesses(&request.command);
        let fingerprint = fingerprint(&self.workspace_id, &op, &request, base, &witnesses)?;
        let policy_decision = self.policy.evaluate(&request.actor, &request.command);
        let approval_required = policy_decision == PolicyDecision::RequireApproval;
        let approval_id = approval_required.then(ApprovalRequestId::new);
        let expires_at = created_at + chrono::Duration::hours(24);
        let reason_codes = if approval_required {
            vec!["agent_mutation_requires_human_review".into()]
        } else {
            vec![]
        };
        let preview = OperationPreview {
            preview_version: 1,
            operation_id: op.clone(),
            workspace_id: self.workspace_id.clone(),
            status: if approval_required {
                OperationStatus::AwaitingApproval
            } else {
                OperationStatus::Proposed
            },
            actor: request.actor.clone(),
            command_type: Self::command_kind(&request.command).into(),
            reason: request.reason.clone(),
            policy_decision,
            required_capabilities: vec![request.command.required_capability()],
            base_schema_revision: base,
            state_witnesses: witnesses.clone(),
            changes: changes.clone(),
            warnings: vec![],
            commit_allowed: !approval_required,
            preview_fingerprint: fingerprint.clone(),
            reversibility: Reversibility::Reversible,
            idempotency_key: request.idempotency_key.clone(),
            correlation: request.correlation.clone(),
            created_at,
            replayed: false,
            approval_request_id: approval_id.clone(),
            expires_at: Some(expires_at),
            policy_reason_codes: reason_codes.clone(),
        };
        let tx = self.conn.transaction().map_err(storage)?;
        upsert_actor(&tx, &request.actor, &created_at.to_rfc3339())?;
        let status = if approval_required {
            "awaiting_approval"
        } else {
            "proposed"
        };
        let policy = if approval_required {
            "require_approval"
        } else {
            "allow"
        };
        tx.execute("INSERT INTO _rowva_operations(id,workspace_id,actor_id,actor_type,actor_display_name,kind,mode,status,reason,idempotency_key,request_json,result_json,created_at,base_schema_revision,resulting_schema_revision,preview_fingerprint,correlation_json,policy_decision,state_witnesses_json,expires_at,policy_revision,policy_reason_codes_json) VALUES(?1,?2,?3,?4,?5,?6,'preview',?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,1,?20)",params![op.as_str(),self.workspace_id.as_str(),request.actor.id.as_str(),actor_type(request.actor.actor_type),request.actor.display_name,Self::command_kind(&request.command),status,request.reason,request.idempotency_key,canonical.to_string(),serde_json::to_string(&preview).map_err(internal)?,created_at.to_rfc3339(),base.0,resulting.0,fingerprint,serde_json::to_string(&request.correlation).map_err(internal)?,policy,serde_json::to_string(&witnesses).map_err(internal)?,expires_at.to_rfc3339(),serde_json::to_string(&reason_codes).map_err(internal)?]).map_err(constraint)?;
        if let Some(id) = &approval_id {
            tx.execute("INSERT INTO _rowva_approval_requests(id,operation_id,workspace_id,proposal_fingerprint,status,required_capability,policy_decision,policy_reason_codes_json,policy_revision,requested_actor_id,requested_at,expires_at) VALUES(?1,?2,?3,?4,'pending',?5,'require_approval',?6,1,?7,?8,?9)", params![id.as_str(),op.as_str(),self.workspace_id.as_str(),fingerprint,capability_name(request.command.required_capability()),serde_json::to_string(&reason_codes).map_err(internal)?,request.actor.id.as_str(),created_at.to_rfc3339(),expires_at.to_rfc3339()]).map_err(storage)?;
        }
        persist_changes(&tx, &op, &changes)?;
        tx.commit().map_err(storage)?;
        Ok(preview)
    }

    fn commit_preview(
        &mut self,
        actor: &ActorContext,
        operation_id: &OperationId,
        fingerprint_value: &str,
    ) -> Result<OperationReceipt, RowvaError> {
        let row:Option<(String,String,String,String,i64,String,String)>=self.conn.query_row("SELECT actor_id,status,request_json,result_json,base_schema_revision,preview_fingerprint,created_at FROM _rowva_operations WHERE id=?1",[operation_id.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional().map_err(storage)?;
        let (actor_id, status, request_json, result_json, base, fingerprint_stored, created) =
            row.ok_or_else(|| RowvaError::not_found("operation", operation_id.as_str()))?;
        if actor_id != actor.id.as_str() {
            return Err(RowvaError::PermissionDenied {
                code: "preview_actor_mismatch".into(),
                message: "configured actor does not own this preview".into(),
            });
        }
        if status == "committed" {
            let mut receipt: OperationReceipt =
                serde_json::from_str(&result_json).map_err(internal)?;
            receipt.replayed = true;
            return Ok(receipt);
        }
        if status == "awaiting_approval" {
            return Err(RowvaError::ApprovalRequired {
                code: "approval_required".into(),
                message: "human approval is required before this proposal can execute".into(),
                operation_id: Some(operation_id.clone()),
            });
        }
        if status != "proposed" {
            return Err(RowvaError::Conflict {
                code: "operation_not_committable".into(),
                message: format!("operation status '{}' cannot be committed", status),
                details: None,
            });
        }
        if fingerprint_value != fingerprint_stored {
            return Err(RowvaError::Conflict {
                code: "preview_fingerprint_mismatch".into(),
                message: "preview fingerprint does not match".into(),
                details: None,
            });
        }
        let request: OperationRequest = serde_json::from_str(&request_json).map_err(internal)?;
        self.policy.authorize(
            actor,
            request.command.required_capability(),
            OperationMode::Commit,
        )?;
        if self.schema_revision()?.0 != base {
            return self.mark_conflict(
                operation_id,
                "schema_revision_conflict",
                "workspace schema changed after preview",
            );
        }
        for witness in state_witnesses(&request.command) {
            let current: Option<i64> = self
                .conn
                .query_row(
                    "SELECT revision FROM _rowva_records WHERE id=?1",
                    [witness.record_id.as_str()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            if current != Some(witness.expected_revision.0) {
                return self.mark_conflict(
                    operation_id,
                    "record_revision_conflict",
                    "record changed after preview",
                );
            }
        }
        let tx = self.conn.transaction().map_err(storage)?;
        let (changes, result, result_schema) = apply_command(
            &tx,
            &self.workspace_id,
            &request.command,
            SchemaRevision(base),
        )?;
        let committed_at = Utc::now();
        let affected = affected_records(&request.command, &result)?;
        let receipt = OperationReceipt {
            receipt_version: 1,
            operation_id: operation_id.clone(),
            workspace_id: self.workspace_id.clone(),
            actor: actor.clone(),
            command_type: Self::command_kind(&request.command).into(),
            status: OperationStatus::Committed,
            reason: request.reason.clone(),
            policy_decision: PolicyDecision::Allow,
            base_schema_revision: SchemaRevision(base),
            result_schema_revision: result_schema,
            affected_records: affected,
            changes: changes.clone(),
            warnings: vec![],
            idempotency_key: request.idempotency_key.clone(),
            correlation: request.correlation.clone(),
            created_at: parse_time(created),
            committed_at,
            reversibility: Reversibility::Reversible,
            replayed: false,
        };
        tx.execute("UPDATE _rowva_operations SET mode='commit',status='committed',result_json=?1,committed_at=?2,resulting_schema_revision=?3 WHERE id=?4 AND status='proposed'",params![serde_json::to_string(&receipt).map_err(internal)?,committed_at.to_rfc3339(),result_schema.0,operation_id.as_str()]).map_err(storage)?;
        tx.execute(
            "DELETE FROM _rowva_operation_changes WHERE operation_id=?1",
            [operation_id.as_str()],
        )
        .map_err(storage)?;
        persist_changes(&tx, operation_id, &changes)?;
        let reverts: Option<String> = tx
            .query_row(
                "SELECT reverts_operation_id FROM _rowva_operations WHERE id=?1",
                [operation_id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(storage)?
            .flatten();
        if let Some(original) = reverts {
            tx.execute("UPDATE _rowva_operations SET status='reverted',reverted_by_operation_id=?1 WHERE id=?2 AND status='committed'", params![operation_id.as_str(), original]).map_err(storage)?;
        }
        tx.commit().map_err(storage)?;
        Ok(receipt)
    }

    fn list_approvals(&mut self, limit: usize) -> Result<Vec<ApprovalRequest>, RowvaError> {
        self.expire_proposals()?;
        let ids = {
            let mut stmt = self.conn.prepare("SELECT id FROM _rowva_approval_requests WHERE workspace_id=?1 ORDER BY requested_at DESC,id DESC LIMIT ?2").map_err(storage)?;
            let rows = stmt
                .query_map(
                    params![self.workspace_id.as_str(), limit.min(100) as i64],
                    |r| r.get::<_, String>(0),
                )
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?;
            rows
        };
        ids.into_iter()
            .map(|id| self.load_approval(&ApprovalRequestId::from_string(id)?))
            .collect()
    }

    fn get_approval(&mut self, id: &ApprovalRequestId) -> Result<ApprovalRequest, RowvaError> {
        self.expire_proposals()?;
        self.load_approval(id)
    }

    fn approve_and_execute(
        &mut self,
        id: &ApprovalRequestId,
        fingerprint_value: &str,
        approver: &ActorContext,
        reason: Option<String>,
    ) -> Result<OperationReceipt, RowvaError> {
        self.policy.authorize(
            approver,
            Capability::OperationsApprove,
            OperationMode::Commit,
        )?;
        validate_decision_reason(&reason)?;
        let approval = self.get_approval(id)?;
        if approval.requested_by.id == approver.id || approver.actor_type != ActorType::Human {
            return Err(RowvaError::PermissionDenied {
                code: "self_approval_denied".into(),
                message: "a human actor distinct from the proposer must approve".into(),
            });
        }
        if approval.proposal_fingerprint != fingerprint_value {
            return Err(RowvaError::Conflict {
                code: "approval_fingerprint_mismatch".into(),
                message: "approval does not match the reviewed proposal".into(),
                details: None,
            });
        }
        if approval.status == ApprovalRequestStatus::Executed {
            let mut receipt: OperationReceipt =
                serde_json::from_value(approval.operation.result.ok_or_else(|| {
                    RowvaError::Internal {
                        code: "missing_receipt".into(),
                    }
                })?)
                .map_err(internal)?;
            receipt.replayed = true;
            return Ok(receipt);
        }
        if approval.status != ApprovalRequestStatus::Pending {
            return Err(RowvaError::Conflict {
                code: "approval_not_pending".into(),
                message: "approval request is no longer pending".into(),
                details: None,
            });
        }
        let now = Utc::now();
        let decision_id = ApprovalDecisionId::new();
        let tx = self.conn.transaction().map_err(storage)?;
        upsert_actor(&tx, approver, &now.to_rfc3339())?;
        tx.execute("INSERT INTO _rowva_approval_decisions(id,approval_request_id,operation_id,proposal_fingerprint,decided_by_actor_id,decision,reason,decided_at) VALUES(?1,?2,?3,?4,?5,'approve',?6,?7)",params![decision_id.as_str(),id.as_str(),approval.operation_id.as_str(),fingerprint_value,approver.id.as_str(),reason,now.to_rfc3339()]).map_err(constraint)?;
        tx.execute("UPDATE _rowva_approval_requests SET status='approved',decided_at=?1 WHERE id=?2 AND status='pending'",params![now.to_rfc3339(),id.as_str()]).map_err(storage)?;
        tx.execute("UPDATE _rowva_operations SET status='proposed' WHERE id=?1 AND status='awaiting_approval'",[approval.operation_id.as_str()]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        match self.commit_preview(
            &approval.requested_by,
            &approval.operation_id,
            fingerprint_value,
        ) {
            Ok(receipt) => {
                self.conn.execute("UPDATE _rowva_approval_requests SET status='executed',executed_at=?1 WHERE id=?2",params![Utc::now().to_rfc3339(),id.as_str()]).map_err(storage)?;
                Ok(receipt)
            }
            Err(error) => {
                let status = if matches!(error, RowvaError::Conflict { .. }) {
                    "execution_conflicted"
                } else {
                    "execution_failed"
                };
                self.conn.execute("UPDATE _rowva_approval_requests SET status=?1,execution_error_json=?2 WHERE id=?3",params![status,serde_json::to_string(&error).map_err(internal)?,id.as_str()]).map_err(storage)?;
                Err(error)
            }
        }
    }

    fn reject_approval(
        &mut self,
        id: &ApprovalRequestId,
        approver: &ActorContext,
        reason: Option<String>,
    ) -> Result<ApprovalRequest, RowvaError> {
        self.policy.authorize(
            approver,
            Capability::OperationsReject,
            OperationMode::Commit,
        )?;
        validate_decision_reason(&reason)?;
        let approval = self.get_approval(id)?;
        if approval.requested_by.id == approver.id || approver.actor_type != ActorType::Human {
            return Err(RowvaError::PermissionDenied {
                code: "self_approval_denied".into(),
                message: "the proposer cannot reject its own request".into(),
            });
        }
        if approval.status == ApprovalRequestStatus::Rejected {
            return Ok(approval);
        }
        if approval.status != ApprovalRequestStatus::Pending {
            return Err(RowvaError::Conflict {
                code: "approval_not_pending".into(),
                message: "approval request is no longer pending".into(),
                details: None,
            });
        }
        let now = Utc::now();
        let decision = ApprovalDecisionId::new();
        let tx = self.conn.transaction().map_err(storage)?;
        upsert_actor(&tx, approver, &now.to_rfc3339())?;
        tx.execute("INSERT INTO _rowva_approval_decisions(id,approval_request_id,operation_id,proposal_fingerprint,decided_by_actor_id,decision,reason,decided_at) VALUES(?1,?2,?3,?4,?5,'reject',?6,?7)",params![decision.as_str(),id.as_str(),approval.operation_id.as_str(),approval.proposal_fingerprint,approver.id.as_str(),reason,now.to_rfc3339()]).map_err(constraint)?;
        tx.execute(
            "UPDATE _rowva_approval_requests SET status='rejected',decided_at=?1 WHERE id=?2",
            params![now.to_rfc3339(), id.as_str()],
        )
        .map_err(storage)?;
        tx.execute("UPDATE _rowva_operations SET status='rejected' WHERE id=?1 AND status='awaiting_approval'",[approval.operation_id.as_str()]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        self.load_approval(id)
    }

    fn revise_approval(
        &mut self,
        id: &ApprovalRequestId,
        editor: &ActorContext,
        replacement_values: HashMap<FieldId, Value>,
        reason: Option<String>,
    ) -> Result<RevisionResult, RowvaError> {
        self.policy
            .authorize(editor, Capability::OperationsRevise, OperationMode::Preview)?;
        let original = self.get_approval(id)?;
        if original.status != ApprovalRequestStatus::Pending {
            return Err(RowvaError::Conflict {
                code: "approval_not_pending".into(),
                message: "only pending proposals can be revised".into(),
                details: None,
            });
        }
        let mut request: OperationRequest =
            serde_json::from_value(original.operation.request.clone()).map_err(internal)?;
        match &mut request.command {
            Command::CreateRecord { values, .. } | Command::UpdateRecord { values, .. } => {
                *values = replacement_values
            }
            _ => {
                return Err(RowvaError::validation(
                    "operation_not_revisable",
                    "only record create and update proposals can be revised",
                ))
            }
        }
        request.reason = reason;
        request.idempotency_key = None;
        let successor = self.preview_operation(request)?;
        let tx = self.conn.transaction().map_err(storage)?;
        tx.execute("UPDATE _rowva_operations SET status='superseded',superseded_by_operation_id=?1 WHERE id=?2 AND status='awaiting_approval'",params![successor.operation_id.as_str(),original.operation_id.as_str()]).map_err(storage)?;
        tx.execute(
            "UPDATE _rowva_operations SET supersedes_operation_id=?1 WHERE id=?2",
            params![
                original.operation_id.as_str(),
                successor.operation_id.as_str()
            ],
        )
        .map_err(storage)?;
        tx.execute("UPDATE _rowva_approval_requests SET status='superseded',decided_at=?1 WHERE id=?2 AND status='pending'",params![Utc::now().to_rfc3339(),id.as_str()]).map_err(storage)?;
        tx.commit().map_err(storage)?;
        Ok(RevisionResult {
            original_operation_id: original.operation_id,
            successor,
        })
    }

    fn preview_undo(
        &mut self,
        operation_id: &OperationId,
        actor: &ActorContext,
        reason: Option<String>,
    ) -> Result<OperationPreview, RowvaError> {
        self.policy
            .authorize(actor, Capability::OperationsUndo, OperationMode::Preview)?;
        let original = self.get_operation(operation_id)?;
        if original.kind != "update_record"
            || original.status != OperationStatus::Committed
            || original.reverted_by_operation_id.is_some()
        {
            return Err(RowvaError::Conflict {
                code: "operation_not_undoable".into(),
                message: "only an unreverted committed record update can be undone".into(),
                details: None,
            });
        }
        let request: OperationRequest =
            serde_json::from_value(original.request.clone()).map_err(internal)?;
        let (object_id, record_id) = match request.command {
            Command::UpdateRecord {
                object_id,
                record_id,
                ..
            } => (object_id, record_id),
            _ => unreachable!(),
        };
        let current = self.get_record(&object_id, &record_id)?;
        let receipt: OperationReceipt =
            serde_json::from_value(original.result.clone().ok_or_else(|| {
                RowvaError::Internal {
                    code: "missing_receipt".into(),
                }
            })?)
            .map_err(internal)?;
        let after_revision = receipt
            .affected_records
            .first()
            .map(|v| v.after_revision)
            .ok_or_else(|| RowvaError::Internal {
                code: "missing_record_revision".into(),
            })?;
        if current.revision != after_revision {
            return Err(RowvaError::Conflict {
                code: "undo_state_conflict".into(),
                message: "the record changed after the original operation".into(),
                details: Some(json!({"recovery":{"action":"review_current_record"}})),
            });
        }
        let mut values = HashMap::new();
        for change in &original.changes {
            if let Some(field) = &change.field_id {
                if current.values.get(field).cloned().unwrap_or(Value::Null)
                    != change.after.clone().unwrap_or(Value::Null)
                {
                    return Err(RowvaError::Conflict {
                        code: "undo_state_conflict".into(),
                        message: "a changed field no longer matches the committed value".into(),
                        details: None,
                    });
                }
                values.insert(field.clone(), change.before.clone().unwrap_or(Value::Null));
            }
        }
        let mut undo = OperationRequest::new(
            actor.clone(),
            Command::UpdateRecord {
                object_id,
                record_id,
                values,
                expected_revision: Some(current.revision),
            },
        );
        undo.reason = reason;
        let preview = self.preview_operation(undo)?;
        self.conn
            .execute(
                "UPDATE _rowva_operations SET reverts_operation_id=?1 WHERE id=?2",
                params![operation_id.as_str(), preview.operation_id.as_str()],
            )
            .map_err(storage)?;
        Ok(preview)
    }

    fn search_records(&self, request: &RecordSearch) -> Result<RecordPage, RowvaError> {
        if request.limit == 0 || request.limit > 100 {
            return Err(RowvaError::validation(
                "result_limit_exceeded",
                "search limit must be between 1 and 100",
            ));
        }
        if request.filters.len() > 10 {
            return Err(RowvaError::validation(
                "too_many_filters",
                "at most 10 filters are allowed",
            ));
        }
        let fields = self.list_fields(&request.object_id)?;
        if fields.is_empty()
            && !self
                .list_objects()?
                .iter()
                .any(|o| o.id == request.object_id)
        {
            return Err(RowvaError::not_found("object", request.object_id.as_str()));
        }
        for f in &request.filters {
            if !fields.iter().any(|x| x.id == f.field_id) {
                return Err(RowvaError::validation(
                    "field_not_found",
                    format!("field '{}' is not in the object", f.field_id),
                ));
            }
        }
        let cursor = request.cursor.as_deref().unwrap_or("");
        let mut sql="SELECT id,object_id,revision,created_at,updated_at FROM _rowva_records r WHERE object_id=?1 AND id>?2".to_string();
        let mut values: Vec<rusqlite::types::Value> = vec![
            request.object_id.to_string().into(),
            cursor.to_owned().into(),
        ];
        for filter in &request.filters {
            sql.push_str(" AND EXISTS(SELECT 1 FROM _rowva_record_values v WHERE v.record_id=r.id AND v.field_id=?");
            values.push(filter.field_id.to_string().into());
            match filter.operator {
                FilterOperator::Equals => {
                    sql.push_str(" AND json_extract(v.value_json,'$')=json_extract(?,'$')");
                    values.push(
                        filter
                            .value
                            .clone()
                            .unwrap_or(Value::Null)
                            .to_string()
                            .into(),
                    );
                }
                FilterOperator::NotEquals => {
                    sql.push_str(" AND json_extract(v.value_json,'$')!=json_extract(?,'$')");
                    values.push(
                        filter
                            .value
                            .clone()
                            .unwrap_or(Value::Null)
                            .to_string()
                            .into(),
                    );
                }
                FilterOperator::Contains => {
                    sql.push_str(" AND instr(CAST(json_extract(v.value_json,'$') AS TEXT),?)>0");
                    let text = filter
                        .value
                        .as_ref()
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            RowvaError::validation(
                                "invalid_filter_value",
                                "contains requires a string value",
                            )
                        })?;
                    values.push(text.to_owned().into());
                }
                FilterOperator::IsNull => sql.push_str(" AND json_type(v.value_json,'$')='null'"),
                FilterOperator::IsNotNull => {
                    sql.push_str(" AND json_type(v.value_json,'$')!='null'")
                }
            }
            sql.push(')');
        }
        sql.push_str(" ORDER BY id LIMIT ?");
        values.push(((request.limit + 1) as i64).into());
        let mut stmt = self.conn.prepare(&sql).map_err(storage)?;
        let mut records = stmt
            .query_map(rusqlite::params_from_iter(values), record_header)
            .map_err(storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage)?;
        let next = if records.len() > request.limit {
            records.truncate(request.limit);
            records.last().map(|r| r.record_id.to_string())
        } else {
            None
        };
        for record in &mut records {
            record.values = load_values(&self.conn, &record.record_id)?;
        }
        Ok(RecordPage {
            object_id: request.object_id.clone(),
            schema_revision: self.schema_revision()?,
            records,
            next_cursor: next,
        })
    }
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                out.insert(key.clone(), canonical_json(&map[key]));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical_json).collect()),
        other => other.clone(),
    }
}
fn fingerprint(
    workspace: &WorkspaceId,
    operation: &OperationId,
    request: &OperationRequest,
    schema: SchemaRevision,
    witnesses: &[StateWitness],
) -> Result<String, RowvaError> {
    let value = json!({"preview_version":1,"workspace_id":workspace,"operation_id":operation,"actor_id":request.actor.id,"command":request.command,"reason":request.reason,"idempotency_key":request.idempotency_key,"schema_revision":schema,"state_witnesses":witnesses,"required_capability":request.command.required_capability()});
    let bytes = serde_json::to_vec(&canonical_json(&value)).map_err(internal)?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}
fn state_witnesses(command: &Command) -> Vec<StateWitness> {
    match command {
        Command::UpdateRecord {
            record_id,
            expected_revision: Some(revision),
            ..
        }
        | Command::DeleteRecord {
            record_id,
            expected_revision: Some(revision),
            ..
        } => vec![StateWitness {
            record_id: record_id.clone(),
            expected_revision: *revision,
        }],
        _ => vec![],
    }
}
fn validate_correlation(value: &CorrelationMetadata) -> Result<(), RowvaError> {
    let encoded = serde_json::to_vec(value).map_err(internal)?;
    if encoded.len() > 2048 {
        return Err(RowvaError::validation(
            "correlation_too_large",
            "correlation metadata exceeds 2048 bytes",
        ));
    }
    Ok(())
}
fn upsert_actor(tx: &Transaction<'_>, actor: &ActorContext, now: &str) -> Result<(), RowvaError> {
    tx.execute("INSERT OR REPLACE INTO _rowva_actors(id,actor_type,display_name,created_at,updated_at,actor_version,human_principal_id,client_name,session_id) VALUES(?1,?2,?3,COALESCE((SELECT created_at FROM _rowva_actors WHERE id=?1),?4),?4,?5,?6,?7,?8)",params![actor.id.as_str(),actor_type(actor.actor_type),actor.display_name,now,actor.actor_version,actor.human_principal_id,actor.client_name,actor.session_id]).map_err(storage)?;
    Ok(())
}
fn persist_changes(
    tx: &Transaction<'_>,
    operation: &OperationId,
    changes: &[ProposedChange],
) -> Result<(), RowvaError> {
    for c in changes {
        tx.execute("INSERT INTO _rowva_operation_changes(operation_id,object_id,record_id,field_id,before_json,after_json) VALUES(?1,?2,?3,?4,?5,?6)",params![operation.as_str(),c.object_id.as_str(),c.record_id.as_ref().map(RecordId::as_str),c.field_id.as_ref().map(FieldId::as_str),c.before.as_ref().map(Value::to_string),c.after.as_ref().map(Value::to_string)]).map_err(storage)?;
    }
    Ok(())
}
fn load_changes(
    conn: &Connection,
    operation: &OperationId,
) -> Result<Vec<ProposedChange>, RowvaError> {
    let mut stmt=conn.prepare("SELECT object_id,record_id,field_id,before_json,after_json FROM _rowva_operation_changes WHERE operation_id=?1 ORDER BY id").map_err(storage)?;
    let rows = stmt
        .query_map([operation.as_str()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    rows.into_iter()
        .map(|(o, r, f, b, a)| {
            Ok(ProposedChange {
                object_id: ObjectId::from_string(o)?,
                record_id: r.map(RecordId::from_string).transpose()?,
                field_id: f.map(FieldId::from_string).transpose()?,
                before: b
                    .map(|v| serde_json::from_str(&v).map_err(internal))
                    .transpose()?,
                after: a
                    .map(|v| serde_json::from_str(&v).map_err(internal))
                    .transpose()?,
            })
        })
        .collect()
}
fn affected_records(command: &Command, result: &Value) -> Result<Vec<AffectedRecord>, RowvaError> {
    match command {
        Command::CreateRecord {
            object_id,
            record_id,
            ..
        } => Ok(vec![AffectedRecord {
            object_id: object_id.clone(),
            record_id: record_id.clone().ok_or_else(|| RowvaError::Internal {
                code: "missing_record_id".into(),
            })?,
            before_revision: None,
            after_revision: RecordRevision(1),
        }]),
        Command::UpdateRecord {
            object_id,
            record_id,
            expected_revision,
            ..
        } => Ok(vec![AffectedRecord {
            object_id: object_id.clone(),
            record_id: record_id.clone(),
            before_revision: *expected_revision,
            after_revision: RecordRevision(result["revision"].as_i64().ok_or_else(|| {
                RowvaError::Internal {
                    code: "missing_record_revision".into(),
                }
            })?),
        }]),
        _ => Ok(vec![]),
    }
}

fn apply_command(
    tx: &Transaction<'_>,
    workspace: &WorkspaceId,
    command: &Command,
    base: SchemaRevision,
) -> Result<(Vec<ProposedChange>, Value, SchemaRevision), RowvaError> {
    let now = Utc::now().to_rfc3339();
    match command {
        Command::CreateObject { display_name, key } => {
            if display_name.trim().is_empty() {
                return Err(RowvaError::validation(
                    "empty_object_name",
                    "object display name is required",
                ));
            }
            let key = key
                .clone()
                .unwrap_or_else(|| SqliteApplication::slug(display_name));
            if key.is_empty() {
                return Err(RowvaError::validation(
                    "empty_object_key",
                    "object key is required",
                ));
            }
            let id = ObjectId::new();
            let rev = SchemaRevision(base.0 + 1);
            tx.execute("INSERT INTO _rowva_objects(id,display_name,key,created_at,updated_at,schema_revision) VALUES(?1,?2,?3,?4,?4,?5)",params![id.as_str(),display_name.trim(),key,now,rev.0]).map_err(constraint)?;
            bump_schema(tx, workspace, rev, &now)?;
            Ok((vec![], json!({"object_id":id}), rev))
        }
        Command::CreateField {
            object_id,
            display_name,
            key,
            kind,
            required,
            unique,
        } => {
            ensure_object(tx, object_id)?;
            if display_name.trim().is_empty() {
                return Err(RowvaError::validation(
                    "empty_field_name",
                    "field display name is required",
                ));
            }
            let id = FieldId::new();
            let key = key
                .clone()
                .unwrap_or_else(|| SqliteApplication::slug(display_name));
            let rev = SchemaRevision(base.0 + 1);
            let kind = serde_json::to_string(kind).map_err(internal)?;
            tx.execute("INSERT INTO _rowva_fields(id,object_id,display_name,key,kind_json,required,unique_flag,position,created_at,updated_at,schema_revision) VALUES(?1,?2,?3,?4,?5,?6,?7,COALESCE((SELECT MAX(position)+1 FROM _rowva_fields WHERE object_id=?2),0),?8,?8,?9)",params![id.as_str(),object_id.as_str(),display_name.trim(),key,kind,required,unique,now,rev.0]).map_err(constraint)?;
            bump_schema(tx, workspace, rev, &now)?;
            Ok((vec![], json!({"field_id":id}), rev))
        }
        Command::CreateRecord {
            object_id,
            record_id,
            values,
        } => {
            ensure_object(tx, object_id)?;
            validate_fields(tx, object_id, values)?;
            let id = record_id.clone().unwrap_or_default();
            tx.execute("INSERT INTO _rowva_records(id,object_id,revision,created_at,updated_at) VALUES(?1,?2,1,?3,?3)",params![id.as_str(),object_id.as_str(),now]).map_err(storage)?;
            let changes = write_values(tx, object_id, &id, values, &HashMap::new())?;
            Ok((
                changes,
                json!({"record_id":id,"revision":1,"values":values}),
                base,
            ))
        }
        Command::UpdateRecord {
            object_id,
            record_id,
            values,
            expected_revision,
        } => {
            validate_fields(tx, object_id, values)?;
            let current: Option<i64> = tx
                .query_row(
                    "SELECT revision FROM _rowva_records WHERE id=?1 AND object_id=?2",
                    params![record_id.as_str(), object_id.as_str()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            let current =
                current.ok_or_else(|| RowvaError::not_found("record", record_id.as_str()))?;
            if expected_revision.is_some_and(|r| r.0 != current) {
                return Err(RowvaError::Conflict {
                    code: "record_revision_conflict".into(),
                    message: format!(
                        "expected revision {:?}, current revision {}",
                        expected_revision, current
                    ),
                    details: Some(json!({"expected":expected_revision,"current":current})),
                });
            }
            let before = load_values_tx(tx, record_id)?;
            let changes = write_values(tx, object_id, record_id, values, &before)?;
            let revision = current + 1;
            tx.execute(
                "UPDATE _rowva_records SET revision=?1,updated_at=?2 WHERE id=?3",
                params![revision, now, record_id.as_str()],
            )
            .map_err(storage)?;
            let mut merged = before;
            merged.extend(values.clone());
            Ok((
                changes,
                json!({"record_id":record_id,"revision":revision,"values":merged}),
                base,
            ))
        }
        Command::DeleteRecord {
            object_id,
            record_id,
            expected_revision,
        } => {
            let current: Option<i64> = tx
                .query_row(
                    "SELECT revision FROM _rowva_records WHERE id=?1 AND object_id=?2",
                    params![record_id.as_str(), object_id.as_str()],
                    |r| r.get(0),
                )
                .optional()
                .map_err(storage)?;
            let current =
                current.ok_or_else(|| RowvaError::not_found("record", record_id.as_str()))?;
            if expected_revision.is_some_and(|r| r.0 != current) {
                return Err(RowvaError::Conflict {
                    code: "record_revision_conflict".into(),
                    message: "record changed before delete".into(),
                    details: Some(json!({"current":current})),
                });
            }
            let before = load_values_tx(tx, record_id)?;
            let changes = before
                .iter()
                .map(|(f, v)| ProposedChange {
                    object_id: object_id.clone(),
                    record_id: Some(record_id.clone()),
                    field_id: Some(f.clone()),
                    before: Some(v.clone()),
                    after: None,
                })
                .collect();
            tx.execute(
                "DELETE FROM _rowva_records WHERE id=?1",
                [record_id.as_str()],
            )
            .map_err(storage)?;
            Ok((changes, json!({"record_id":record_id,"deleted":true}), base))
        }
    }
}

fn write_values(
    tx: &Transaction<'_>,
    object: &ObjectId,
    record: &RecordId,
    values: &HashMap<FieldId, Value>,
    before: &HashMap<FieldId, Value>,
) -> Result<Vec<ProposedChange>, RowvaError> {
    let mut out = vec![];
    for (f, v) in values {
        tx.execute("INSERT INTO _rowva_record_values(record_id,field_id,value_json) VALUES(?1,?2,?3) ON CONFLICT(record_id,field_id) DO UPDATE SET value_json=excluded.value_json",params![record.as_str(),f.as_str(),v.to_string()]).map_err(storage)?;
        out.push(ProposedChange {
            object_id: object.clone(),
            record_id: Some(record.clone()),
            field_id: Some(f.clone()),
            before: before.get(f).cloned(),
            after: Some(v.clone()),
        });
    }
    Ok(out)
}
fn validate_fields(
    tx: &Transaction<'_>,
    object: &ObjectId,
    values: &HashMap<FieldId, Value>,
) -> Result<(), RowvaError> {
    for f in values.keys() {
        let ok: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _rowva_fields WHERE id=?1 AND object_id=?2)",
                params![f.as_str(), object.as_str()],
                |r| r.get(0),
            )
            .map_err(storage)?;
        if !ok {
            return Err(RowvaError::validation(
                "unknown_field",
                format!("field '{}' does not belong to object", f),
            ));
        }
    }
    Ok(())
}
fn ensure_object(tx: &Transaction<'_>, id: &ObjectId) -> Result<(), RowvaError> {
    let ok: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _rowva_objects WHERE id=?1)",
            [id.as_str()],
            |r| r.get(0),
        )
        .map_err(storage)?;
    if ok {
        Ok(())
    } else {
        Err(RowvaError::not_found("object", id.as_str()))
    }
}
fn bump_schema(
    tx: &Transaction<'_>,
    workspace: &WorkspaceId,
    rev: SchemaRevision,
    now: &str,
) -> Result<(), RowvaError> {
    tx.execute(
        "UPDATE _rowva_workspace SET schema_revision=?1,updated_at=?2 WHERE id=?3",
        params![rev.0, now, workspace.as_str()],
    )
    .map_err(storage)?;
    Ok(())
}
fn load_values(conn: &Connection, id: &RecordId) -> Result<HashMap<FieldId, Value>, RowvaError> {
    let mut s = conn
        .prepare("SELECT field_id,value_json FROM _rowva_record_values WHERE record_id=?1")
        .map_err(storage)?;
    let rows = s
        .query_map([id.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(storage)?;
    parse_values(rows.collect::<Result<Vec<_>, _>>().map_err(storage)?)
}
fn load_values_tx(
    tx: &Transaction<'_>,
    id: &RecordId,
) -> Result<HashMap<FieldId, Value>, RowvaError> {
    let mut s = tx
        .prepare("SELECT field_id,value_json FROM _rowva_record_values WHERE record_id=?1")
        .map_err(storage)?;
    let rows = s
        .query_map([id.as_str()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(storage)?;
    parse_values(rows.collect::<Result<Vec<_>, _>>().map_err(storage)?)
}
fn parse_values(rows: Vec<(String, String)>) -> Result<HashMap<FieldId, Value>, RowvaError> {
    rows.into_iter()
        .map(|(f, v)| {
            Ok((
                FieldId::from_string(f)?,
                serde_json::from_str(&v).map_err(internal)?,
            ))
        })
        .collect()
}

fn object_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<ObjectDefinition> {
    Ok(ObjectDefinition {
        id: ObjectId::from_string(r.get::<_, String>(0)?).map_err(sql_conversion)?,
        display_name: r.get(1)?,
        key: r.get(2)?,
        created_at: parse_time(r.get::<_, String>(3)?),
        updated_at: parse_time(r.get::<_, String>(4)?),
        schema_revision: SchemaRevision(r.get(5)?),
    })
}
fn field_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<FieldDefinition> {
    let kind: String = r.get(4)?;
    Ok(FieldDefinition {
        id: FieldId::from_string(r.get::<_, String>(0)?).map_err(sql_conversion)?,
        object_id: ObjectId::from_string(r.get::<_, String>(1)?).map_err(sql_conversion)?,
        display_name: r.get(2)?,
        key: r.get(3)?,
        kind: serde_json::from_str(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?,
        required: r.get(5)?,
        unique: r.get(6)?,
        created_at: parse_time(r.get(7)?),
        updated_at: parse_time(r.get(8)?),
        schema_revision: SchemaRevision(r.get(9)?),
    })
}
fn record_header(r: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        record_id: RecordId::from_string(r.get::<_, String>(0)?).map_err(sql_conversion)?,
        object_id: ObjectId::from_string(r.get::<_, String>(1)?).map_err(sql_conversion)?,
        revision: RecordRevision(r.get(2)?),
        values: HashMap::new(),
        created_at: parse_time(r.get(3)?),
        updated_at: parse_time(r.get(4)?),
    })
}
fn operation_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let actor_kind: String = r.get(2)?;
    let request: String = r.get(9)?;
    let result: Option<String> = r.get(10)?;
    let error: Option<String> = r.get(11)?;
    Ok(OperationRecord {
        id: OperationId::from_string(r.get::<_, String>(0)?).map_err(sql_conversion)?,
        actor: ActorContext {
            id: ActorId::from_string(r.get::<_, String>(1)?).map_err(sql_conversion)?,
            actor_type: parse_actor(&actor_kind),
            display_name: r.get(3)?,
            capabilities: vec![],
            actor_version: None,
            human_principal_id: None,
            client_name: None,
            session_id: None,
        },
        kind: r.get(4)?,
        mode: parse_mode(&r.get::<_, String>(5)?),
        status: parse_status(&r.get::<_, String>(6)?),
        reason: r.get(7)?,
        idempotency_key: r.get(8)?,
        request: serde_json::from_str(&request).unwrap_or(Value::Null),
        result: result.and_then(|v| serde_json::from_str(&v).ok()),
        error: error.and_then(|v| serde_json::from_str(&v).ok()),
        created_at: parse_time(r.get(12)?),
        committed_at: r.get::<_, Option<String>>(13)?.map(parse_time),
        base_schema_revision: SchemaRevision(r.get(14)?),
        resulting_schema_revision: r.get::<_, Option<i64>>(15)?.map(SchemaRevision),
        changes: vec![],
        correlation: r
            .get::<_, Option<String>>(16)?
            .and_then(|v| serde_json::from_str(&v).ok())
            .unwrap_or_default(),
        policy_decision: r.get::<_, Option<String>>(17)?.map(|v| {
            if v == "allow" {
                PolicyDecision::Allow
            } else if v == "require_approval" {
                PolicyDecision::RequireApproval
            } else {
                PolicyDecision::Deny
            }
        }),
        preview_fingerprint: r.get(18)?,
        expires_at: None,
        supersedes_operation_id: None,
        superseded_by_operation_id: None,
        reverts_operation_id: None,
        reverted_by_operation_id: None,
    })
}
fn parse_time(v: String) -> chrono::DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(&v)
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
fn actor_type(v: ActorType) -> &'static str {
    match v {
        ActorType::Human => "human",
        ActorType::Agent => "agent",
        ActorType::Integration => "integration",
        ActorType::System => "system",
    }
}
fn parse_actor(v: &str) -> ActorType {
    match v {
        "agent" => ActorType::Agent,
        "integration" => ActorType::Integration,
        "system" => ActorType::System,
        _ => ActorType::Human,
    }
}
fn capability_name(v: Capability) -> &'static str {
    match v {
        Capability::RecordsCreate => "records.create",
        Capability::RecordsUpdate => "records.update",
        Capability::OperationsApprove => "operations.approve",
        Capability::OperationsReject => "operations.reject",
        Capability::OperationsRevise => "operations.revise",
        Capability::OperationsUndo => "operations.undo",
        Capability::ApprovalsRead => "approvals.read",
        _ => "operations.read",
    }
}
fn parse_capability(v: &str) -> Capability {
    match v {
        "records.create" => Capability::RecordsCreate,
        "records.update" => Capability::RecordsUpdate,
        "operations.reject" => Capability::OperationsReject,
        "operations.revise" => Capability::OperationsRevise,
        "operations.undo" => Capability::OperationsUndo,
        "approvals.read" => Capability::ApprovalsRead,
        _ => Capability::OperationsApprove,
    }
}
fn parse_approval_status(v: &str) -> ApprovalRequestStatus {
    match v {
        "approved" => ApprovalRequestStatus::Approved,
        "rejected" => ApprovalRequestStatus::Rejected,
        "expired" => ApprovalRequestStatus::Expired,
        "superseded" => ApprovalRequestStatus::Superseded,
        "executed" => ApprovalRequestStatus::Executed,
        "execution_conflicted" => ApprovalRequestStatus::ExecutionConflicted,
        "execution_failed" => ApprovalRequestStatus::ExecutionFailed,
        _ => ApprovalRequestStatus::Pending,
    }
}
fn validate_decision_reason(reason: &Option<String>) -> Result<(), RowvaError> {
    if reason.as_ref().is_some_and(|v| v.len() > 2048) {
        Err(RowvaError::validation(
            "reason_too_long",
            "decision reason exceeds 2048 bytes",
        ))
    } else {
        Ok(())
    }
}
fn mode(v: OperationMode) -> &'static str {
    if v == OperationMode::Preview {
        "preview"
    } else {
        "commit"
    }
}
fn parse_mode(v: &str) -> OperationMode {
    if v == "preview" {
        OperationMode::Preview
    } else {
        OperationMode::Commit
    }
}
fn parse_status(v: &str) -> OperationStatus {
    match v {
        "proposed" => OperationStatus::Proposed,
        "awaiting_approval" => OperationStatus::AwaitingApproval,
        "rejected" => OperationStatus::Rejected,
        "failed" => OperationStatus::Failed,
        "reverted" => OperationStatus::Reverted,
        "conflicted" => OperationStatus::Conflicted,
        "expired" => OperationStatus::Expired,
        "superseded" => OperationStatus::Superseded,
        _ => OperationStatus::Committed,
    }
}
fn constraint(e: rusqlite::Error) -> RowvaError {
    if matches!(e, rusqlite::Error::SqliteFailure(_, _)) {
        RowvaError::Conflict {
            code: "unique_constraint".into(),
            message: "a stable key already exists in this scope".into(),
            details: None,
        }
    } else {
        storage(e)
    }
}
fn storage(error: rusqlite::Error) -> RowvaError {
    let busy = matches!(&error,rusqlite::Error::SqliteFailure(failure,_) if matches!(failure.code,rusqlite::ErrorCode::DatabaseBusy|rusqlite::ErrorCode::DatabaseLocked));
    RowvaError::Storage {
        code: if busy { "storage_busy" } else { "sqlite_error" }.into(),
    }
}
fn internal<E: std::fmt::Display>(_: E) -> RowvaError {
    RowvaError::Internal {
        code: "serialization_error".into(),
    }
}
fn sql_conversion(_: RowvaError) -> rusqlite::Error {
    rusqlite::Error::InvalidQuery
}

#[cfg(test)]
mod tests;
