//! Thin Tauri compatibility adapter over the shared Rowva application protocol.
use rowva_core::{
    ActorContext, ApprovalRequest, ApprovalRequestId, Command, FieldDefinition, FieldId, FieldKind,
    ObjectId, OperationId, OperationPreview, OperationReceipt, OperationRecord, OperationRequest,
    RecordId, RecordRevision, RevisionResult, RowvaApplication, RowvaError, TextConfig,
};
use rowva_store_sqlite::SqliteApplication;
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::HashMap, path::Path, sync::Mutex};
use tauri::{Manager, State};

#[derive(Default)]
pub struct AppState {
    current: Mutex<Option<SqliteApplication>>,
}
#[derive(Serialize)]
pub struct GridColumn {
    id: FieldId,
    label: String,
    col_type: FieldKind,
}
#[derive(Serialize)]
pub struct GridData {
    columns: Vec<GridColumn>,
    rows: Vec<Value>,
}

fn lock<'a>(
    state: &'a State<'_, AppState>,
) -> Result<std::sync::MutexGuard<'a, Option<SqliteApplication>>, RowvaError> {
    state.current.lock().map_err(|_| RowvaError::Internal {
        code: "desktop_state_poisoned".into(),
    })
}
fn app_mut(slot: &mut Option<SqliteApplication>) -> Result<&mut SqliteApplication, RowvaError> {
    slot.as_mut()
        .ok_or_else(|| RowvaError::validation("no_workspace_open", "no workspace is open"))
}

#[tauri::command]
pub fn new_workbook(
    state: State<'_, AppState>,
    name: String,
    app: tauri::AppHandle,
) -> Result<String, RowvaError> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| RowvaError::Internal {
            code: "app_data_directory_unavailable".into(),
        })?;
    std::fs::create_dir_all(&data_dir).map_err(|_| RowvaError::Storage {
        code: "create_app_data_directory_failed".into(),
    })?;
    let safe_name = name
        .to_lowercase()
        .replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    let path = data_dir.join(format!("{}.rowva", safe_name.trim_matches('_')));
    *lock(&state)? = Some(SqliteApplication::create(&path, &name)?);
    Ok(path.to_string_lossy().into_owned())
}
#[tauri::command]
pub fn open_workbook(state: State<'_, AppState>, path: String) -> Result<String, RowvaError> {
    *lock(&state)? = Some(SqliteApplication::open(Path::new(&path))?);
    Ok(path)
}
#[tauri::command]
pub fn create_table(
    state: State<'_, AppState>,
    display_name: String,
) -> Result<String, RowvaError> {
    let mut slot = lock(&state)?;
    let response = app_mut(&mut slot)?.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::CreateObject {
            display_name,
            key: None,
        },
    ))?;
    response
        .result
        .and_then(|v| v["object_id"].as_str().map(str::to_owned))
        .ok_or(RowvaError::Internal {
            code: "missing_object_result".into(),
        })
}
#[tauri::command]
pub fn add_column(
    state: State<'_, AppState>,
    table_id: String,
    label: String,
) -> Result<FieldDefinition, RowvaError> {
    let object_id = ObjectId::from_string(table_id)?;
    let mut slot = lock(&state)?;
    let application = app_mut(&mut slot)?;
    application.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::CreateField {
            object_id: object_id.clone(),
            display_name: label,
            key: None,
            kind: FieldKind::Text(TextConfig::default()),
            required: false,
            unique: false,
        },
    ))?;
    application
        .list_fields(&object_id)?
        .pop()
        .ok_or(RowvaError::Internal {
            code: "missing_field_result".into(),
        })
}
#[tauri::command]
pub fn get_tables(state: State<'_, AppState>) -> Result<Vec<Value>, RowvaError> {
    let mut slot = lock(&state)?;
    Ok(app_mut(&mut slot)?
        .list_objects()?
        .into_iter()
        .map(|o| json!({"id":o.id,"display_name":o.display_name}))
        .collect())
}
#[tauri::command]
pub fn get_grid_data(state: State<'_, AppState>, table_id: String) -> Result<GridData, RowvaError> {
    let object_id = ObjectId::from_string(table_id)?;
    let mut slot = lock(&state)?;
    let application = app_mut(&mut slot)?;
    let columns = application
        .list_fields(&object_id)?
        .into_iter()
        .map(|f| GridColumn {
            id: f.id,
            label: f.display_name,
            col_type: f.kind,
        })
        .collect();
    let rows = application
        .list_records(&object_id)?
        .into_iter()
        .map(|r| json!({"record_id":r.record_id,"revision":r.revision,"values":r.values}))
        .collect();
    Ok(GridData { columns, rows })
}
#[tauri::command]
pub fn insert_row(
    state: State<'_, AppState>,
    table_id: String,
    values: HashMap<String, Value>,
) -> Result<String, RowvaError> {
    let object_id = ObjectId::from_string(table_id)?;
    let values = values
        .into_iter()
        .map(|(id, v)| Ok((FieldId::from_string(id)?, v)))
        .collect::<Result<HashMap<_, _>, RowvaError>>()?;
    let mut slot = lock(&state)?;
    let response = app_mut(&mut slot)?.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::CreateRecord {
            object_id,
            record_id: None,
            values,
        },
    ))?;
    response
        .result
        .and_then(|v| v["record_id"].as_str().map(str::to_owned))
        .ok_or(RowvaError::Internal {
            code: "missing_record_result".into(),
        })
}
#[tauri::command]
pub fn update_cell(
    state: State<'_, AppState>,
    table_id: String,
    row_id: String,
    col_id: String,
    value: Value,
    expected_revision: Option<i64>,
) -> Result<Value, RowvaError> {
    let mut values = HashMap::new();
    values.insert(FieldId::from_string(col_id)?, value);
    let mut slot = lock(&state)?;
    let response = app_mut(&mut slot)?.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::UpdateRecord {
            object_id: ObjectId::from_string(table_id)?,
            record_id: RecordId::from_string(row_id)?,
            values,
            expected_revision: expected_revision.map(RecordRevision),
        },
    ))?;
    response.result.ok_or(RowvaError::Internal {
        code: "missing_update_result".into(),
    })
}
#[tauri::command]
pub fn delete_row_cmd(
    state: State<'_, AppState>,
    table_id: String,
    row_id: String,
    expected_revision: Option<i64>,
) -> Result<(), RowvaError> {
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.execute(OperationRequest::new(
        ActorContext::local_user(),
        Command::DeleteRecord {
            object_id: ObjectId::from_string(table_id)?,
            record_id: RecordId::from_string(row_id)?,
            expected_revision: expected_revision.map(RecordRevision),
        },
    ))?;
    Ok(())
}

#[tauri::command]
pub fn list_approval_inbox(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<ApprovalRequest>, RowvaError> {
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.list_approvals(limit.unwrap_or(50))
}
#[tauri::command]
pub fn get_approval_detail(
    state: State<'_, AppState>,
    approval_request_id: String,
) -> Result<ApprovalRequest, RowvaError> {
    let id = ApprovalRequestId::from_string(approval_request_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.get_approval(&id)
}
#[tauri::command]
pub fn approve_and_execute_operation(
    state: State<'_, AppState>,
    approval_request_id: String,
    proposal_fingerprint: String,
    reason: Option<String>,
) -> Result<OperationReceipt, RowvaError> {
    let id = ApprovalRequestId::from_string(approval_request_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.approve_and_execute(
        &id,
        &proposal_fingerprint,
        &ActorContext::local_user(),
        reason,
    )
}
#[tauri::command]
pub fn reject_operation(
    state: State<'_, AppState>,
    approval_request_id: String,
    reason: Option<String>,
) -> Result<ApprovalRequest, RowvaError> {
    let id = ApprovalRequestId::from_string(approval_request_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.reject_approval(&id, &ActorContext::local_user(), reason)
}
#[tauri::command]
pub fn revise_operation(
    state: State<'_, AppState>,
    approval_request_id: String,
    replacement_values: HashMap<String, Value>,
    reason: Option<String>,
) -> Result<RevisionResult, RowvaError> {
    let values = replacement_values
        .into_iter()
        .map(|(id, v)| Ok((FieldId::from_string(id)?, v)))
        .collect::<Result<HashMap<_, _>, RowvaError>>()?;
    let id = ApprovalRequestId::from_string(approval_request_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.revise_approval(&id, &ActorContext::local_user(), values, reason)
}
#[tauri::command]
pub fn preview_operation_undo(
    state: State<'_, AppState>,
    operation_id: String,
    reason: Option<String>,
) -> Result<OperationPreview, RowvaError> {
    let id = OperationId::from_string(operation_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.preview_undo(&id, &ActorContext::local_user(), reason)
}
#[tauri::command]
pub fn commit_operation_undo(
    state: State<'_, AppState>,
    operation_id: String,
    preview_fingerprint: String,
) -> Result<OperationReceipt, RowvaError> {
    let id = OperationId::from_string(operation_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.commit_preview(&ActorContext::local_user(), &id, &preview_fingerprint)
}
#[tauri::command]
pub fn list_operation_history(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<OperationRecord>, RowvaError> {
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.list_operations(limit.unwrap_or(50))
}
#[tauri::command]
pub fn get_operation_detail(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<OperationRecord, RowvaError> {
    let id = OperationId::from_string(operation_id)?;
    let mut slot = lock(&state)?;
    app_mut(&mut slot)?.get_operation(&id)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapter_rejects_display_labels_as_ids() {
        assert!(ObjectId::from_string("Contacts").is_err());
    }
}
