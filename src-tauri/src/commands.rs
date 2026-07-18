use tauri::{State, Manager};
use crate::engine::RowvaWorkbook;     // now works via re-export
use crate::engine::table::{Column, Table};
use crate::storage;
use std::sync::Mutex;
use serde_json::{json, Value};
use anyhow::Error;                    // ← for explicit map_err type

#[derive(Default)]
pub struct AppState {
    current_workbook: Mutex<Option<RowvaWorkbook>>,
}

#[tauri::command]
pub fn new_workbook(state: State<AppState>, name: String, app: tauri::AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let file_path = data_dir.join(format!("{}.rowva", name.to_lowercase().replace(' ', "_")));

    let wb: RowvaWorkbook = RowvaWorkbook::create(name, file_path.clone())
        .map_err(|e: Error| e.to_string())?;   // explicit type fixes E0282

    let mut guard = state.current_workbook.lock().unwrap();
    *guard = Some(wb);
    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn create_table(state: State<AppState>, displayName: String) -> Result<String, String> {
    let mut guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_mut().ok_or("No workbook open".to_string())?;

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    let table = storage::create_table(&conn, &displayName).map_err(|e| e.to_string())?;

    wb.tables.insert(table.id.clone(), table.clone());
    Ok(table.id)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn add_column(state: State<AppState>, tableId: String, label: String) -> Result<Column, String> {
    let mut guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_mut().ok_or("No workbook open".to_string())?;

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    let column = storage::add_column(&conn, &tableId, &label).map_err(|e| e.to_string())?;

    if let Some(table) = wb.tables.get_mut(&tableId) {
        table.columns.push(column.clone());
    }
    Ok(column)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn get_grid_data(state: State<AppState>, tableId: String) -> Result<Value, String> {
    let guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_ref().ok_or("No workbook open".to_string())?;
    let table = wb.tables.get(&tableId).ok_or("Table not found".to_string())?;

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    let rows = storage::get_table_rows(&conn, &table.name, &table.columns).map_err(|e| e.to_string())?;

    let cols_json = table.columns.iter().map(|c| {
        json!({ "id": c.id, "label": c.label, "col_type": "Text" })
    }).collect::<Vec<_>>();

    Ok(json!({ "columns": cols_json, "rows": rows }))
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn insert_row(
    state: State<AppState>,
    tableId: String,
    values: std::collections::HashMap<String, String>, // label -> value (for simplicity in first slice)
) -> Result<String, String> {
    let mut guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_mut().ok_or("No workbook open".to_string())?;
    let table = wb.tables.get(&tableId).ok_or("Table not found".to_string())?;

    // Build map from label -> col_id for this table
    let label_to_col: std::collections::HashMap<_, _> = table
        .columns
        .iter()
        .map(|c| (c.label.clone(), c.id.clone()))
        .collect();

    let mut col_values: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
    for (label, val) in values {
        if let Some(col_id) = label_to_col.get(&label) {
            col_values.insert(col_id.clone(), if val.is_empty() { None } else { Some(val) });
        }
    }

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    let row_id = storage::insert_row(&conn, &table.name, &col_values).map_err(|e| e.to_string())?;

    Ok(row_id)
}

#[tauri::command]
pub fn open_workbook(state: State<AppState>, path: String) -> Result<String, String> {
    let conn = storage::open_workbook(std::path::Path::new(&path)).map_err(|e| e.to_string())?;

    let mut loaded_tables: Vec<Table> = storage::load_all_tables(&conn).map_err(|e| e.to_string())?;

    for table in &mut loaded_tables {
        let cols = storage::load_columns_for_table(&conn, &table.id).map_err(|e| e.to_string())?;
        table.columns = cols;
    }

    let mut tables_map = std::collections::HashMap::new();
    for t in loaded_tables {
        tables_map.insert(t.id.clone(), t);
    }

    let wb = RowvaWorkbook {
        id: uuid::Uuid::new_v4(),
        name: std::path::Path::new(&path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Workbook")
            .to_string(),
        path: std::path::PathBuf::from(&path),
        tables: tables_map,
    };

    let mut guard = state.current_workbook.lock().unwrap();
    *guard = Some(wb);

    Ok(path)
}

#[tauri::command]
pub fn get_tables(state: State<AppState>) -> Result<Vec<serde_json::Value>, String> {
    let guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_ref().ok_or("No workbook open".to_string())?;

    let tables = wb.tables.values().map(|t| {
        serde_json::json!({
            "id": t.id,
            "display_name": t.display_name,
        })
    }).collect();

    Ok(tables)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn update_cell(
    state: State<AppState>,
    tableId: String,
    rowId: String,
    colId: String,   // internal column id (c_xxx)
    value: String,
) -> Result<(), String> {
    let guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_ref().ok_or("No workbook open".to_string())?;
    let table = wb.tables.get(&tableId).ok_or("Table not found".to_string())?;

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    storage::update_cell(&conn, &table.name, &rowId, &colId, if value.is_empty() { None } else { Some(&value) })
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn delete_row_cmd(state: State<AppState>, tableId: String, rowId: String) -> Result<(), String> {
    let guard = state.current_workbook.lock().unwrap();
    let wb = guard.as_ref().ok_or("No workbook open".to_string())?;
    let table = wb.tables.get(&tableId).ok_or("Table not found".to_string())?;

    let conn = storage::open_workbook(&wb.path).map_err(|e| e.to_string())?;
    storage::delete_row(&conn, &table.name, &rowId).map_err(|e| e.to_string())
}