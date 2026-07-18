use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColType {
    Text,
    Number,
    Formula { formula: String },
    Ref { target_table: String, show_column: String },
    RefList { target_table: String },
    Attachment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: String,
    pub label: String,
    pub col_type: ColType,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub columns: Vec<Column>,
}