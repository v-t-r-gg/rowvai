use rusqlite::{Connection, params, Result as SqlResult};
use std::path::Path;
use uuid::Uuid;
use crate::engine::table::{Table, Column, ColType};
use serde_json::{json, Value};

pub fn create_new_workbook(path: &Path) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    eprintln!("DEBUG: create_new_workbook - meta tables created");
    conn.execute_batch(
        r#"
        PRAGMA journal_mode=WAL;
        BEGIN;
        CREATE TABLE IF NOT EXISTS _rowva_tables (id TEXT PRIMARY KEY, name TEXT NOT NULL, display_name TEXT);
        CREATE TABLE IF NOT EXISTS _rowva_columns (id TEXT PRIMARY KEY, table_id TEXT, col_id TEXT, label TEXT, col_type TEXT, position INTEGER DEFAULT 999);
        COMMIT;
        "#,
    )?;
    Ok(conn)
}

pub fn open_workbook(path: &Path) -> SqlResult<Connection> {
    let conn = Connection::open(path)?;
    conn.execute("PRAGMA journal_mode=WAL;", [])?;
    Ok(conn)
}

pub fn create_table(conn: &Connection, display_name: &str) -> SqlResult<Table> {
    eprintln!("=== CREATE_TABLE START: '{}'", display_name);
    let id = Uuid::new_v4().to_string();
    let table_name = format!("t_{}", id.replace('-', "_"));

    eprintln!("=== INSERT meta table '{}'", table_name);
    conn.execute(
        "INSERT INTO _rowva_tables (id, name, display_name) VALUES (?1, ?2, ?3)",
        params![&id, &table_name, display_name],
    )?;

    eprintln!("=== CREATE real table '{}'", table_name);
    let sql = format!(r#"CREATE TABLE "{}" (row_id TEXT PRIMARY KEY)"#, table_name);
    conn.execute(&sql, [])?;

    eprintln!("=== CREATE_TABLE SUCCESS: '{}'", display_name);
    Ok(Table {
        id,
        name: table_name,
        display_name: display_name.to_string(),
        columns: vec![],
    })
}

pub fn add_column(conn: &Connection, table_id: &str, label: &str) -> SqlResult<Column> {
    eprintln!("=== ADD_COLUMN START table_id={} label={}", table_id, label);
    let col_id = format!("c_{}", Uuid::new_v4().to_string().replace('-', "_"));

    let table_name: String = conn.query_row(
        "SELECT name FROM _rowva_tables WHERE id=?1",
        [table_id],
        |r| r.get(0),
    )?;

    eprintln!("=== ALTER TABLE '{}' ADD '{}'", table_name, col_id);
    let alter = format!(r#"ALTER TABLE "{}" ADD COLUMN "{}" TEXT"#, table_name, col_id);
    conn.execute(&alter, [])?;

    eprintln!("=== INSERT meta column '{}'", label);
    conn.execute(
        "INSERT INTO _rowva_columns (id, table_id, col_id, label, col_type, position) VALUES (?1,?2,?3,?4,?5,?6)",
        params![Uuid::new_v4().to_string(), table_id, &col_id, label, "Text", 999i32],
    )?;

    eprintln!("=== ADD_COLUMN SUCCESS");
    Ok(Column { id: col_id, label: label.to_string(), col_type: ColType::Text, position: 999 })
}

pub fn get_table_rows(conn: &Connection, table_name: &str, columns: &[Column]) -> SqlResult<Vec<Value>> {
    let mut col_list = vec!["row_id".to_string()];
    col_list.extend(columns.iter().map(|c| format!(r#""{}""#, c.id)));
    let sql = format!(r#"SELECT {} FROM "{}""#, col_list.join(", "), table_name);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        let mut obj = serde_json::Map::new();
        obj.insert("row_id".to_string(), json!(row.get::<_, String>(0)?));
        for (i, col) in columns.iter().enumerate() {
            let val: Option<String> = row.get(i + 1)?;
            obj.insert(col.label.clone(), json!(val.unwrap_or_default()));
        }
        Ok(Value::Object(obj))
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Insert a new row into the given table.
/// `values` is a map from internal column id (e.g. "c_xxx") to the cell value.
pub fn insert_row(
    conn: &Connection,
    table_name: &str,
    values: &std::collections::HashMap<String, Option<String>>,
) -> SqlResult<String> {
    let row_id = Uuid::new_v4().to_string();

    let mut col_names: Vec<String> = vec!["row_id".to_string()];
    let mut placeholders: Vec<String> = vec!["?1".to_string()];
    let mut param_values: Vec<Option<String>> = vec![Some(row_id.clone())];

    for (col_id, val) in values {
        col_names.push(col_id.clone());
        placeholders.push(format!("?{}", param_values.len() + 1));
        param_values.push(val.clone());
    }

    let sql = format!(
        r#"INSERT INTO "{}" ({}) VALUES ({})"#,
        table_name,
        col_names
            .iter()
            .map(|c| format!(r#""{}""#, c))
            .collect::<Vec<_>>()
            .join(", "),
        placeholders.join(", ")
    );

    // Convert to a slice of &dyn ToSql
    let params: Vec<&dyn rusqlite::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::ToSql)
        .collect();

    conn.execute(&sql, &*params)?;

    Ok(row_id)
}

/// Update a single cell in a row.
/// `col_id` is the internal column id (e.g. "c_xxx").
pub fn update_cell(
    conn: &Connection,
    table_name: &str,
    row_id: &str,
    col_id: &str,
    value: Option<&str>,
) -> SqlResult<()> {
    let sql = format!(r#"UPDATE "{}" SET "{}" = ?1 WHERE row_id = ?2"#, table_name, col_id);
    conn.execute(&sql, params![value, row_id])?;
    Ok(())
}

/// Delete a row by its row_id.
pub fn delete_row(conn: &Connection, table_name: &str, row_id: &str) -> SqlResult<()> {
    let sql = format!(r#"DELETE FROM "{}" WHERE row_id = ?1"#, table_name);
    conn.execute(&sql, params![row_id])?;
    Ok(())
}

// --- Loading existing workbooks ---

pub fn load_all_tables(conn: &Connection) -> SqlResult<Vec<Table>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, display_name FROM _rowva_tables ORDER BY display_name"
    )?;
    let tables = stmt.query_map([], |row| {
        Ok(Table {
            id: row.get(0)?,
            name: row.get(1)?,
            display_name: row.get(2)?,
            columns: vec![], // will be filled later
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(tables)
}

pub fn load_columns_for_table(conn: &Connection, table_id: &str) -> SqlResult<Vec<Column>> {
    let mut stmt = conn.prepare(
        r#"SELECT col_id, label, col_type, position 
           FROM _rowva_columns 
           WHERE table_id = ?1 
           ORDER BY position, label"#
    )?;
    let columns = stmt.query_map([table_id], |row| {
        let col_type_str: String = row.get(2)?;
        let col_type = match col_type_str.as_str() {
            "Number" => ColType::Number,
            "Formula" => ColType::Formula { formula: String::new() }, // placeholder
            "Ref" => ColType::Ref { target_table: String::new(), show_column: String::new() },
            "RefList" => ColType::RefList { target_table: String::new() },
            "Attachment" => ColType::Attachment,
            _ => ColType::Text,
        };

        Ok(Column {
            id: row.get(0)?,
            label: row.get(1)?,
            col_type,
            position: row.get(3)?,
        })
    })?.collect::<Result<Vec<_>, _>>()?;
    Ok(columns)
}