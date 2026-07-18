// Grist-style Ref resolution stub (full impl in Phase 4)
use rusqlite::{Connection, Result as SqlResult};

pub fn resolve_ref(
    _conn: &Connection,
    _ref_id: &str,
    _target_table: &str,
    _show_col: &str,
) -> SqlResult<Option<String>> {
    Ok(None) // placeholder
}