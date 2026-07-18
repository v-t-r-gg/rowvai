use crate::engine::table::Table;
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

pub struct RowvaWorkbook {          // Debug removed (external engine in Phase 3)
    pub id: Uuid,
    pub name: String,
    pub path: PathBuf,
    pub tables: HashMap<String, Table>,
}

impl RowvaWorkbook {
    pub fn create(name: String, path: PathBuf) -> anyhow::Result<Self> {
        crate::storage::create_new_workbook(&path)?;

        Ok(Self {
            id: Uuid::new_v4(),
            name,
            path,
            tables: HashMap::new(),
        })
    }
}