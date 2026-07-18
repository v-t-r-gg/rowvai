//! Transport- and storage-neutral Rowva domain and application protocol.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, fmt, hash::Hash};
use thiserror::Error;
use uuid::Uuid;

mod evaluation;
pub use evaluation::*;

macro_rules! id_type {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn new() -> Self {
                Self(format!(concat!($prefix, "_{}"), Uuid::new_v4().simple()))
            }
            pub fn from_string(value: impl Into<String>) -> Result<Self, RowvaError> {
                let value = value.into();
                if value.starts_with(concat!($prefix, "_")) && value.len() > 4 {
                    Ok(Self(value))
                } else {
                    Err(RowvaError::validation(
                        "invalid_id",
                        format!("invalid {} identifier", $prefix),
                    ))
                }
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(WorkspaceId, "wsp");
id_type!(ObjectId, "obj");
id_type!(FieldId, "fld");
id_type!(RecordId, "rec");
id_type!(ActorId, "act");
id_type!(OperationId, "op");
id_type!(ApprovalRequestId, "apr");
id_type!(ApprovalDecisionId, "apd");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SchemaRevision(pub i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordRevision(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Human,
    Agent,
    Integration,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorContext {
    pub id: ActorId,
    pub actor_type: ActorType,
    pub display_name: String,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub actor_version: Option<String>,
    #[serde(default)]
    pub human_principal_id: Option<String>,
    #[serde(default)]
    pub client_name: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}
impl ActorContext {
    pub fn local_user() -> Self {
        Self {
            id: ActorId::from_string("act_local_user").unwrap_or_default(),
            actor_type: ActorType::Human,
            display_name: "Local User".into(),
            capabilities: Capability::all(),
            actor_version: None,
            human_principal_id: None,
            client_name: Some("rowva-desktop".into()),
            session_id: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    WorkspaceRead,
    SchemaRead,
    SchemaWrite,
    RecordsRead,
    RecordsCreate,
    RecordsUpdate,
    RecordsDelete,
    OperationsRead,
    OperationsApprove,
    OperationsReject,
    OperationsRevise,
    OperationsUndo,
    ApprovalsRead,
}
impl Capability {
    pub fn all() -> Vec<Self> {
        vec![
            Self::WorkspaceRead,
            Self::SchemaRead,
            Self::SchemaWrite,
            Self::RecordsRead,
            Self::RecordsCreate,
            Self::RecordsUpdate,
            Self::RecordsDelete,
            Self::OperationsRead,
            Self::OperationsApprove,
            Self::OperationsReject,
            Self::OperationsRevise,
            Self::OperationsUndo,
            Self::ApprovalsRead,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "snake_case")]
pub enum FieldKind {
    Text(TextConfig),
    Number(NumberConfig),
    Boolean,
    Date,
    DateTime,
    Enum(EnumConfig),
    Relation(RelationConfig),
    RelationList(RelationConfig),
    Computed(ComputedConfig),
    Attachment,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextConfig {
    pub max_length: Option<u32>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NumberConfig {
    pub precision: Option<u8>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnumConfig {
    pub options: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationConfig {
    pub target_object_id: ObjectId,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputedConfig {
    pub expression: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectDefinition {
    pub id: ObjectId,
    pub display_name: String,
    pub key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_revision: SchemaRevision,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub id: FieldId,
    pub object_id: ObjectId,
    pub display_name: String,
    pub key: String,
    pub kind: FieldKind,
    pub required: bool,
    pub unique: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub schema_revision: SchemaRevision,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub record_id: RecordId,
    pub object_id: ObjectId,
    pub revision: RecordRevision,
    pub values: HashMap<FieldId, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationMode {
    Preview,
    Commit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Proposed,
    AwaitingApproval,
    Committed,
    Rejected,
    Failed,
    Reverted,
    Conflicted,
    Expired,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "input", rename_all = "snake_case")]
pub enum Command {
    CreateObject {
        display_name: String,
        key: Option<String>,
    },
    CreateField {
        object_id: ObjectId,
        display_name: String,
        key: Option<String>,
        kind: FieldKind,
        required: bool,
        unique: bool,
    },
    CreateRecord {
        object_id: ObjectId,
        #[serde(default)]
        record_id: Option<RecordId>,
        values: HashMap<FieldId, Value>,
    },
    UpdateRecord {
        object_id: ObjectId,
        record_id: RecordId,
        values: HashMap<FieldId, Value>,
        expected_revision: Option<RecordRevision>,
    },
    DeleteRecord {
        object_id: ObjectId,
        record_id: RecordId,
        expected_revision: Option<RecordRevision>,
    },
}
impl Command {
    pub fn required_capability(&self) -> Capability {
        match self {
            Self::CreateObject { .. } | Self::CreateField { .. } => Capability::SchemaWrite,
            Self::CreateRecord { .. } => Capability::RecordsCreate,
            Self::UpdateRecord { .. } => Capability::RecordsUpdate,
            Self::DeleteRecord { .. } => Capability::RecordsDelete,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRequest {
    pub protocol_version: u16,
    pub actor: ActorContext,
    pub command: Command,
    pub reason: Option<String>,
    pub idempotency_key: Option<String>,
    pub mode: OperationMode,
    #[serde(default)]
    pub correlation: CorrelationMetadata,
}
impl OperationRequest {
    pub fn new(actor: ActorContext, command: Command) -> Self {
        Self {
            protocol_version: 1,
            actor,
            command,
            reason: None,
            idempotency_key: None,
            mode: OperationMode::Commit,
            correlation: CorrelationMetadata::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedChange {
    pub object_id: ObjectId,
    pub record_id: Option<RecordId>,
    pub field_id: Option<FieldId>,
    pub before: Option<Value>,
    pub after: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationResponse {
    pub protocol_version: u16,
    pub operation_id: OperationId,
    pub status: OperationStatus,
    pub warnings: Vec<ProtocolWarning>,
    pub changes: Vec<ProposedChange>,
    pub result: Option<Value>,
    pub base_schema_revision: SchemaRevision,
    pub resulting_schema_revision: SchemaRevision,
    pub replayed: bool,
    pub undo_token: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrelationMetadata {
    pub external_run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub trace_parent: Option<String>,
    pub client_request_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allow,
    Deny,
    RequireApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    Reversible,
    Compensatable,
    Irreversible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateWitness {
    pub record_id: RecordId,
    pub expected_revision: RecordRevision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationPreview {
    pub preview_version: u16,
    pub operation_id: OperationId,
    pub workspace_id: WorkspaceId,
    pub status: OperationStatus,
    pub actor: ActorContext,
    pub command_type: String,
    pub reason: Option<String>,
    pub policy_decision: PolicyDecision,
    pub required_capabilities: Vec<Capability>,
    pub base_schema_revision: SchemaRevision,
    pub state_witnesses: Vec<StateWitness>,
    pub changes: Vec<ProposedChange>,
    pub warnings: Vec<ProtocolWarning>,
    pub commit_allowed: bool,
    pub preview_fingerprint: String,
    pub reversibility: Reversibility,
    pub idempotency_key: Option<String>,
    pub correlation: CorrelationMetadata,
    pub created_at: DateTime<Utc>,
    pub replayed: bool,
    #[serde(default)]
    pub approval_request_id: Option<ApprovalRequestId>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub policy_reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Superseded,
    Executed,
    ExecutionConflicted,
    ExecutionFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionKind {
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub id: ApprovalRequestId,
    pub operation_id: OperationId,
    pub workspace_id: WorkspaceId,
    pub proposal_fingerprint: String,
    pub status: ApprovalRequestStatus,
    pub requested_by: ActorContext,
    pub required_approver_capability: Capability,
    pub policy_decision: PolicyDecision,
    pub policy_reason_codes: Vec<String>,
    pub policy_revision: i64,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub executed_at: Option<DateTime<Utc>>,
    pub operation: OperationRecord,
    pub decision: Option<ApprovalDecision>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub id: ApprovalDecisionId,
    pub approval_request_id: ApprovalRequestId,
    pub operation_id: OperationId,
    pub proposal_fingerprint: String,
    pub decided_by: ActorContext,
    pub decision: ApprovalDecisionKind,
    pub reason: Option<String>,
    pub decided_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevisionResult {
    pub original_operation_id: OperationId,
    pub successor: OperationPreview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AffectedRecord {
    pub object_id: ObjectId,
    pub record_id: RecordId,
    pub before_revision: Option<RecordRevision>,
    pub after_revision: RecordRevision,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationReceipt {
    pub receipt_version: u16,
    pub operation_id: OperationId,
    pub workspace_id: WorkspaceId,
    pub actor: ActorContext,
    pub command_type: String,
    pub status: OperationStatus,
    pub reason: Option<String>,
    pub policy_decision: PolicyDecision,
    pub base_schema_revision: SchemaRevision,
    pub result_schema_revision: SchemaRevision,
    pub affected_records: Vec<AffectedRecord>,
    pub changes: Vec<ProposedChange>,
    pub warnings: Vec<ProtocolWarning>,
    pub idempotency_key: Option<String>,
    pub correlation: CorrelationMetadata,
    pub created_at: DateTime<Utc>,
    pub committed_at: DateTime<Utc>,
    pub reversibility: Reversibility,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Equals,
    NotEquals,
    Contains,
    IsNull,
    IsNotNull,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordFilter {
    pub field_id: FieldId,
    pub operator: FilterOperator,
    pub value: Option<Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordSearch {
    pub object_id: ObjectId,
    pub filters: Vec<RecordFilter>,
    pub limit: usize,
    pub cursor: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordPage {
    pub object_id: ObjectId,
    pub schema_revision: SchemaRevision,
    pub records: Vec<Record>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub id: OperationId,
    pub actor: ActorContext,
    pub kind: String,
    pub mode: OperationMode,
    pub status: OperationStatus,
    pub reason: Option<String>,
    pub idempotency_key: Option<String>,
    pub request: Value,
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
    pub base_schema_revision: SchemaRevision,
    pub resulting_schema_revision: Option<SchemaRevision>,
    #[serde(default)]
    pub changes: Vec<ProposedChange>,
    #[serde(default)]
    pub correlation: CorrelationMetadata,
    pub policy_decision: Option<PolicyDecision>,
    pub preview_fingerprint: Option<String>,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub supersedes_operation_id: Option<OperationId>,
    #[serde(default)]
    pub superseded_by_operation_id: Option<OperationId>,
    #[serde(default)]
    pub reverts_operation_id: Option<OperationId>,
    #[serde(default)]
    pub reverted_by_operation_id: Option<OperationId>,
}

#[derive(Debug, Error, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum RowvaError {
    #[error("{message}")]
    NotFound { code: String, message: String },
    #[error("{message}")]
    Validation {
        code: String,
        message: String,
        details: Option<Value>,
    },
    #[error("{message}")]
    Conflict {
        code: String,
        message: String,
        details: Option<Value>,
    },
    #[error("{message}")]
    PermissionDenied { code: String, message: String },
    #[error("{message}")]
    ApprovalRequired {
        code: String,
        message: String,
        operation_id: Option<OperationId>,
    },
    #[error("storage operation failed")]
    Storage { code: String },
    #[error("internal operation failed")]
    Internal { code: String },
}
impl RowvaError {
    pub fn validation(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }
    pub fn not_found(kind: &str, id: &str) -> Self {
        Self::NotFound {
            code: format!("{}_not_found", kind),
            message: format!("{} '{}' was not found", kind, id),
        }
    }
}

pub trait Policy: Send + Sync {
    fn authorize(
        &self,
        actor: &ActorContext,
        capability: Capability,
        mode: OperationMode,
    ) -> Result<(), RowvaError>;
    fn evaluate(&self, actor: &ActorContext, command: &Command) -> PolicyDecision {
        match (actor.actor_type, command) {
            (
                ActorType::Agent | ActorType::Integration,
                Command::CreateRecord { .. } | Command::UpdateRecord { .. },
            ) => PolicyDecision::RequireApproval,
            _ => PolicyDecision::Allow,
        }
    }
}
#[derive(Default)]
pub struct CapabilityPolicy;
impl Policy for CapabilityPolicy {
    fn authorize(
        &self,
        actor: &ActorContext,
        capability: Capability,
        _mode: OperationMode,
    ) -> Result<(), RowvaError> {
        if actor.capabilities.contains(&capability) {
            Ok(())
        } else {
            Err(RowvaError::PermissionDenied {
                code: "missing_capability".into(),
                message: format!("actor lacks {:?}", capability),
            })
        }
    }
}

pub trait RowvaApplication {
    fn execute(&mut self, request: OperationRequest) -> Result<OperationResponse, RowvaError>;
    fn list_objects(&self) -> Result<Vec<ObjectDefinition>, RowvaError>;
    fn list_fields(&self, object_id: &ObjectId) -> Result<Vec<FieldDefinition>, RowvaError>;
    fn list_records(&self, object_id: &ObjectId) -> Result<Vec<Record>, RowvaError>;
    fn get_record(&self, object_id: &ObjectId, record_id: &RecordId) -> Result<Record, RowvaError>;
    fn get_operation(&self, operation_id: &OperationId) -> Result<OperationRecord, RowvaError>;
    fn list_operations(&self, limit: usize) -> Result<Vec<OperationRecord>, RowvaError>;
    fn preview_operation(
        &mut self,
        request: OperationRequest,
    ) -> Result<OperationPreview, RowvaError>;
    fn commit_preview(
        &mut self,
        actor: &ActorContext,
        operation_id: &OperationId,
        fingerprint: &str,
    ) -> Result<OperationReceipt, RowvaError>;
    fn search_records(&self, request: &RecordSearch) -> Result<RecordPage, RowvaError>;
    fn list_approvals(&mut self, limit: usize) -> Result<Vec<ApprovalRequest>, RowvaError>;
    fn get_approval(&mut self, id: &ApprovalRequestId) -> Result<ApprovalRequest, RowvaError>;
    fn approve_and_execute(
        &mut self,
        id: &ApprovalRequestId,
        fingerprint: &str,
        approver: &ActorContext,
        reason: Option<String>,
    ) -> Result<OperationReceipt, RowvaError>;
    fn reject_approval(
        &mut self,
        id: &ApprovalRequestId,
        approver: &ActorContext,
        reason: Option<String>,
    ) -> Result<ApprovalRequest, RowvaError>;
    fn revise_approval(
        &mut self,
        id: &ApprovalRequestId,
        editor: &ActorContext,
        replacement_values: HashMap<FieldId, Value>,
        reason: Option<String>,
    ) -> Result<RevisionResult, RowvaError>;
    fn preview_undo(
        &mut self,
        operation_id: &OperationId,
        actor: &ActorContext,
        reason: Option<String>,
    ) -> Result<OperationPreview, RowvaError>;
}
