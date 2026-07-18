//! MCP translation layer. Domain decisions remain in `RowvaApplication`.
use rmcp::{
    handler::server::wrapper::Parameters, model::CallToolResult, schemars, tool, tool_router,
};
use rowva_core::*;
use rowva_store_sqlite::SqliteApplication;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing::{info, warn};

pub const DEFAULT_PAGE_SIZE: usize = 25;
pub const MAX_PAGE_SIZE: usize = 100;
pub const MAX_FILTERS: usize = 10;
pub const MAX_FIELD_CHANGES: usize = 100;

#[derive(Debug, Clone)]
pub struct ServerIdentity {
    pub actor: ActorContext,
    pub actor_version: Option<String>,
    pub session_id: String,
}

#[derive(Clone)]
pub struct RowvaMcp {
    application: Arc<Mutex<SqliteApplication>>,
    identity: ServerIdentity,
}

#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaInput {
    pub object_id: Option<String>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecordGetInput {
    pub object_id: String,
    pub record_id: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchFilterInput {
    pub field_id: String,
    pub operator: String,
    pub value: Option<Value>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchInput {
    pub object_id: String,
    #[serde(default)]
    pub filters: Vec<SearchFilterInput>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum MutationInput {
    CreateRecord {
        object_id: String,
        values: HashMap<String, Value>,
    },
    UpdateRecord {
        object_id: String,
        record_id: String,
        expected_revision: i64,
        values: HashMap<String, Value>,
    },
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreviewInput {
    pub command: MutationInput,
    pub reason: Option<String>,
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub correlation: CorrelationInput,
}
#[derive(Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CorrelationInput {
    pub external_run_id: Option<String>,
    pub workflow_id: Option<String>,
    pub source_type: Option<String>,
    pub source_id: Option<String>,
    pub trace_parent: Option<String>,
    pub client_request_id: Option<String>,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitInput {
    pub operation_id: String,
    pub preview_fingerprint: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationGetInput {
    pub operation_id: String,
}
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationListInput {
    pub status: Option<String>,
    pub operation_kind: Option<String>,
    pub actor_id: Option<String>,
    pub object_id: Option<String>,
    pub record_id: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

impl RowvaMcp {
    pub fn new(application: SqliteApplication, identity: ServerIdentity) -> Self {
        Self {
            application: Arc::new(Mutex::new(application)),
            identity,
        }
    }
    fn app(&self) -> Result<MutexGuard<'_, SqliteApplication>, RowvaError> {
        self.application.lock().map_err(|_| RowvaError::Internal {
            code: "application_state_poisoned".into(),
        })
    }
    fn require(&self, capability: Capability) -> Result<(), RowvaError> {
        CapabilityPolicy
            .authorize(&self.identity.actor, capability, OperationMode::Preview)
            .map_err(|_| RowvaError::PermissionDenied {
                code: "capability_denied".into(),
                message: format!("configured actor lacks {}", capability_name(capability)),
            })
    }
    fn result<T: serde::Serialize>(
        &self,
        tool: &str,
        result: Result<T, RowvaError>,
    ) -> CallToolResult {
        match result {
            Ok(value) => {
                info!(event="mcp.tool.result",tool_name=tool,status="success",actor_id=%self.identity.actor.id,session_id=%self.identity.session_id);
                match serde_json::to_value(value) {
                    Ok(value) => CallToolResult::structured(value),
                    Err(_) => CallToolResult::structured_error(error_envelope(
                        RowvaError::Internal {
                            code: "internal_error".into(),
                        },
                        None,
                    )),
                }
            }
            Err(error) => {
                warn!(event="mcp.tool.result",tool_name=tool,status="error",error_code=error_code(&error),actor_id=%self.identity.actor.id,session_id=%self.identity.session_id);
                CallToolResult::structured_error(error_envelope(error, None))
            }
        }
    }
    fn map_command(&self, input: MutationInput) -> Result<Command, RowvaError> {
        match input {
            MutationInput::CreateRecord { object_id, values } => {
                if values.len() > MAX_FIELD_CHANGES {
                    return Err(RowvaError::validation(
                        "too_many_field_changes",
                        "at most 100 fields may change",
                    ));
                }
                Ok(Command::CreateRecord {
                    object_id: ObjectId::from_string(object_id)?,
                    record_id: None,
                    values: parse_values(values)?,
                })
            }
            MutationInput::UpdateRecord {
                object_id,
                record_id,
                expected_revision,
                values,
            } => {
                if values.len() > MAX_FIELD_CHANGES {
                    return Err(RowvaError::validation(
                        "too_many_field_changes",
                        "at most 100 fields may change",
                    ));
                }
                Ok(Command::UpdateRecord {
                    object_id: ObjectId::from_string(object_id)?,
                    record_id: RecordId::from_string(record_id)?,
                    expected_revision: Some(RecordRevision(expected_revision)),
                    values: parse_values(values)?,
                })
            }
        }
    }
}

#[tool_router(server_handler)]
impl RowvaMcp {
    #[tool(
        name = "rowva_workspace_describe",
        description = "Describe the single RowvAI workspace bound to this process without exposing its filesystem path.",
        annotations(
            title = "Describe RowvAI workspace",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn workspace_describe(&self, Parameters(_): Parameters<EmptyInput>) -> CallToolResult {
        let result = (|| {
            self.require(Capability::WorkspaceRead)?;
            let app = self.app()?;
            let objects = app.list_objects()?;
            Ok(
                json!({"protocol_response_version":1,"workspace_id":app.workspace_id(),"workspace_display_name":app.workspace_name()?,"workspace_format_version":3,"schema_revision":app.schema_revision()?,"available_object_count":objects.len(),"configured_actor":{"actor_id":self.identity.actor.id,"actor_type":self.identity.actor.actor_type,"actor_name":self.identity.actor.display_name,"actor_version":self.identity.actor_version,"session_id":self.identity.session_id},"effective_capabilities":self.identity.actor.capabilities.iter().map(|c|capability_name(*c)).collect::<Vec<_>>(),"supported_operation_kinds":["create_record","update_record"],"server_version":env!("CARGO_PKG_VERSION"),"features":{"durable_previews":true,"approval_execution":false,"undo":false,"delete":false}}),
            )
        })();
        self.result("rowva_workspace_describe", result)
    }

    #[tool(
        name = "rowva_schema_describe",
        description = "Describe all logical CRM objects and complete safe field definitions, or one object by immutable ID.",
        annotations(
            title = "Describe RowvAI schema",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn schema_describe(&self, Parameters(input): Parameters<SchemaInput>) -> CallToolResult {
        let result = (|| {
            self.require(Capability::SchemaRead)?;
            let app = self.app()?;
            let wanted = input.object_id.map(ObjectId::from_string).transpose()?;
            let mut objects = app.list_objects()?;
            if let Some(id) = &wanted {
                objects.retain(|o| &o.id == id);
                if objects.is_empty() {
                    return Err(RowvaError::not_found("object", id.as_str()));
                }
            }
            let described=objects.into_iter().map(|object|{let fields=app.list_fields(&object.id)?;Ok(json!({"object":object,"fields":fields,"supported_mutations":["create_record","update_record"]}))}).collect::<Result<Vec<Value>,RowvaError>>()?;
            Ok(
                json!({"protocol_response_version":1,"schema_revision":app.schema_revision()?,"objects":described}),
            )
        })();
        self.result("rowva_schema_describe", result)
    }

    #[tool(
        name = "rowva_records_search",
        description = "Search records with bounded typed filters. SQL fragments and filesystem paths are not accepted.",
        annotations(
            title = "Search RowvAI records",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn records_search(&self, Parameters(input): Parameters<SearchInput>) -> CallToolResult {
        let result = (|| {
            self.require(Capability::RecordsRead)?;
            if input.filters.len() > MAX_FILTERS {
                return Err(RowvaError::validation(
                    "too_many_filters",
                    "at most 10 filters are allowed",
                ));
            }
            let filters = input
                .filters
                .into_iter()
                .map(|f| {
                    Ok(RecordFilter {
                        field_id: FieldId::from_string(f.field_id)?,
                        operator: parse_operator(&f.operator)?,
                        value: f.value,
                    })
                })
                .collect::<Result<Vec<_>, RowvaError>>()?;
            self.app()?.search_records(&RecordSearch {
                object_id: ObjectId::from_string(input.object_id)?,
                filters,
                limit: input.limit.unwrap_or(DEFAULT_PAGE_SIZE),
                cursor: input.cursor,
            })
        })();
        self.result("rowva_records_search", result)
    }

    #[tool(
        name = "rowva_record_get",
        description = "Retrieve one record by immutable object and record IDs with field-ID-keyed values.",
        annotations(
            title = "Get RowvAI record",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn record_get(&self, Parameters(input): Parameters<RecordGetInput>) -> CallToolResult {
        let result = (|| {
            self.require(Capability::RecordsRead)?;
            let app = self.app()?;
            let object = ObjectId::from_string(input.object_id)?;
            let record = app.get_record(&object, &RecordId::from_string(input.record_id)?)?;
            Ok(
                json!({"protocol_response_version":1,"schema_revision":app.schema_revision()?,"object_id":object,"record":record}),
            )
        })();
        self.result("rowva_record_get", result)
    }

    #[tool(
        name = "rowva_operation_preview",
        description = "Persist a state-bound proposal for create_record or update_record without changing business state.",
        annotations(
            title = "Preview RowvAI operation",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn operation_preview(
        &self,
        Parameters(input): Parameters<PreviewInput>,
    ) -> CallToolResult {
        let result = (|| {
            let command = self.map_command(input.command)?;
            let mut request = OperationRequest::new(self.identity.actor.clone(), command);
            request.mode = OperationMode::Preview;
            request.reason = input.reason;
            request.idempotency_key = input.idempotency_key;
            request.correlation = input.correlation.into();
            let preview = self.app()?.preview_operation(request)?;
            info!(event="rowva.operation.preview",operation_id=%preview.operation_id,actor_id=%self.identity.actor.id,status=?preview.status);
            Ok(preview)
        })();
        self.result("rowva_operation_preview", result)
    }

    #[tool(
        name = "rowva_operation_commit",
        description = "Commit the exact stored preview identified by operation ID and state-bound fingerprint. No replacement command is accepted.",
        annotations(
            title = "Commit RowvAI operation",
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn operation_commit(&self, Parameters(input): Parameters<CommitInput>) -> CallToolResult {
        let result = (|| {
            let operation = OperationId::from_string(input.operation_id)?;
            let receipt = self.app()?.commit_preview(
                &self.identity.actor,
                &operation,
                &input.preview_fingerprint,
            )?;
            info!(event="rowva.operation.commit",operation_id=%receipt.operation_id,actor_id=%self.identity.actor.id,status=?receipt.status);
            Ok(receipt)
        })();
        self.result("rowva_operation_commit", result)
    }

    #[tool(
        name = "rowva_operation_get",
        description = "Inspect one durable operation owned by the configured actor.",
        annotations(
            title = "Get RowvAI operation",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn operation_get(
        &self,
        Parameters(input): Parameters<OperationGetInput>,
    ) -> CallToolResult {
        let result = (|| {
            self.require(Capability::OperationsRead)?;
            let operation = self
                .app()?
                .get_operation(&OperationId::from_string(input.operation_id)?)?;
            if operation.actor.id != self.identity.actor.id {
                return Err(RowvaError::PermissionDenied {
                    code: "operation_visibility_denied".into(),
                    message: "configured policy permits viewing only this actor's operations"
                        .into(),
                });
            }
            Ok(operation)
        })();
        self.result("rowva_operation_get", result)
    }

    #[tool(
        name = "rowva_operation_list",
        description = "List concise operation history owned by the configured actor with bounded filtering.",
        annotations(
            title = "List RowvAI operations",
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn operation_list(
        &self,
        Parameters(input): Parameters<OperationListInput>,
    ) -> CallToolResult {
        let result = (|| {
            self.require(Capability::OperationsRead)?;
            let limit = input.limit.unwrap_or(DEFAULT_PAGE_SIZE);
            if limit == 0 || limit > MAX_PAGE_SIZE {
                return Err(RowvaError::validation(
                    "result_limit_exceeded",
                    "operation limit must be between 1 and 100",
                ));
            }
            if input
                .actor_id
                .as_ref()
                .is_some_and(|id| id != self.identity.actor.id.as_str())
            {
                return Err(RowvaError::PermissionDenied {
                    code: "operation_visibility_denied".into(),
                    message: "configured policy permits viewing only this actor's operations"
                        .into(),
                });
            }
            let mut operations = self.app()?.list_operations(MAX_PAGE_SIZE + 1)?;
            operations.retain(|o| o.actor.id == self.identity.actor.id);
            if let Some(status) = input.status {
                operations.retain(|o| format!("{:?}", o.status).to_lowercase() == status);
            }
            if let Some(kind) = input.operation_kind {
                operations.retain(|o| o.kind == kind);
            }
            if let Some(cursor) = input.cursor {
                operations.retain(|o| o.id.as_str() < cursor.as_str());
            }
            if let Some(object) = input.object_id {
                operations.retain(|o| o.changes.iter().any(|c| c.object_id.as_str() == object));
            }
            if let Some(record) = input.record_id {
                operations.retain(|o| {
                    o.changes
                        .iter()
                        .any(|c| c.record_id.as_ref().is_some_and(|r| r.as_str() == record))
                });
            }
            let next = if operations.len() > limit {
                operations.truncate(limit);
                operations.last().map(|o| o.id.to_string())
            } else {
                None
            };
            let summaries=operations.into_iter().map(|o|json!({"operation_id":o.id,"actor_id":o.actor.id,"kind":o.kind,"status":o.status,"mode":o.mode,"reason":o.reason,"created_at":o.created_at,"committed_at":o.committed_at,"base_schema_revision":o.base_schema_revision,"resulting_schema_revision":o.resulting_schema_revision,"idempotency_key":o.idempotency_key})).collect::<Vec<_>>();
            Ok(json!({"protocol_response_version":1,"operations":summaries,"next_cursor":next}))
        })();
        self.result("rowva_operation_list", result)
    }
}

impl From<CorrelationInput> for CorrelationMetadata {
    fn from(v: CorrelationInput) -> Self {
        Self {
            external_run_id: v.external_run_id,
            workflow_id: v.workflow_id,
            source_type: v.source_type,
            source_id: v.source_id,
            trace_parent: v.trace_parent,
            client_request_id: v.client_request_id,
        }
    }
}
fn parse_values(values: HashMap<String, Value>) -> Result<HashMap<FieldId, Value>, RowvaError> {
    values
        .into_iter()
        .map(|(id, v)| Ok((FieldId::from_string(id)?, v)))
        .collect()
}
fn parse_operator(value: &str) -> Result<FilterOperator, RowvaError> {
    match value {
        "equals" => Ok(FilterOperator::Equals),
        "not_equals" => Ok(FilterOperator::NotEquals),
        "contains" => Ok(FilterOperator::Contains),
        "is_null" => Ok(FilterOperator::IsNull),
        "is_not_null" => Ok(FilterOperator::IsNotNull),
        _ => Err(RowvaError::validation(
            "unsupported_filter_operator",
            "supported operators: equals, not_equals, contains, is_null, is_not_null",
        )),
    }
}
pub fn capability_name(value: Capability) -> &'static str {
    match value {
        Capability::WorkspaceRead => "workspace.read",
        Capability::SchemaRead => "schema.read",
        Capability::SchemaWrite => "schema.write",
        Capability::RecordsRead => "records.read",
        Capability::RecordsCreate => "records.create",
        Capability::RecordsUpdate => "records.update",
        Capability::RecordsDelete => "records.delete",
        Capability::OperationsRead => "operations.read",
        Capability::OperationsApprove => "operations.approve",
        Capability::OperationsReject => "operations.reject",
        Capability::OperationsRevise => "operations.revise",
        Capability::OperationsUndo => "operations.undo",
        Capability::ApprovalsRead => "approvals.read",
    }
}
pub fn parse_capability(value: &str) -> Result<Capability, String> {
    match value {
        "workspace.read" => Ok(Capability::WorkspaceRead),
        "schema.read" => Ok(Capability::SchemaRead),
        "records.read" => Ok(Capability::RecordsRead),
        "records.create" => Ok(Capability::RecordsCreate),
        "records.update" => Ok(Capability::RecordsUpdate),
        "records.delete" => Ok(Capability::RecordsDelete),
        "operations.read" => Ok(Capability::OperationsRead),
        _ => Err(format!("unsupported capability '{value}'")),
    }
}
fn error_code(error: &RowvaError) -> &str {
    match error {
        RowvaError::NotFound { code, .. }
        | RowvaError::Validation { code, .. }
        | RowvaError::Conflict { code, .. }
        | RowvaError::PermissionDenied { code, .. }
        | RowvaError::ApprovalRequired { code, .. }
        | RowvaError::Storage { code }
        | RowvaError::Internal { code } => match code.as_str() {
            "invalid_id" => "invalid_input",
            "missing_capability" => "capability_denied",
            "idempotency_key_reused" => "idempotency_conflict",
            other => other,
        },
    }
}
fn error_envelope(error: RowvaError, operation: Option<OperationId>) -> Value {
    let (category, message, retryable, recovery) = match &error {
        RowvaError::NotFound { message, .. } => ("not_found", message.clone(), false, None),
        RowvaError::Validation { message, .. } => ("validation", message.clone(), false, None),
        RowvaError::Conflict { message, .. } => (
            "conflict",
            message.clone(),
            true,
            Some(json!({"action":"preview_again"})),
        ),
        RowvaError::PermissionDenied { message, .. } => {
            ("permission_denied", message.clone(), false, None)
        }
        RowvaError::ApprovalRequired { message, .. } => (
            "approval_required",
            message.clone(),
            false,
            Some(json!({"action":"request_human_approval"})),
        ),
        RowvaError::Storage { .. } => (
            "storage",
            "workspace storage is temporarily unavailable".into(),
            true,
            Some(json!({"action":"retry_later"})),
        ),
        RowvaError::Internal { .. } => ("internal", "internal RowvAI error".into(), false, None),
    };
    json!({"error":{"code":error_code(&error),"category":category,"message":message,"retryable":retryable,"recovery":recovery,"operation_id":operation}})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_fail_closed() {
        assert!(parse_capability("schema.write").is_err());
        assert_eq!(
            parse_capability("records.read").unwrap(),
            Capability::RecordsRead
        );
    }
    #[test]
    fn error_does_not_leak_storage() {
        let value = error_envelope(
            RowvaError::Storage {
                code: "sqlite_error".into(),
            },
            None,
        );
        assert!(!value.to_string().contains("_rowva_"));
    }
    #[test]
    fn tool_schemas_and_annotations_are_safe() {
        let tools = [
            RowvaMcp::workspace_describe_tool_attr(),
            RowvaMcp::schema_describe_tool_attr(),
            RowvaMcp::records_search_tool_attr(),
            RowvaMcp::record_get_tool_attr(),
            RowvaMcp::operation_preview_tool_attr(),
            RowvaMcp::operation_commit_tool_attr(),
            RowvaMcp::operation_get_tool_attr(),
            RowvaMcp::operation_list_tool_attr(),
        ];
        assert_eq!(tools.len(), 8);
        assert!(tools.iter().all(|tool| tool
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.open_world_hint)
            == Some(false)));
        assert_eq!(
            tools[0].annotations.as_ref().unwrap().read_only_hint,
            Some(true)
        );
        assert_eq!(
            tools[4].annotations.as_ref().unwrap().read_only_hint,
            Some(false)
        );
        let commit_schema = serde_json::to_value(&tools[5].input_schema)
            .unwrap()
            .to_string();
        assert!(commit_schema.contains("preview_fingerprint"));
        assert!(!commit_schema.contains("command"));
    }
}
