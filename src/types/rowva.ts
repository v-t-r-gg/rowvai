// Core Rowva domain types (mirroring Rust engine/table.rs)

export type ColType =
  | 'Text'
  | 'Number'
  | { Formula: { formula: string } }
  | { Ref: { target_table: string; show_column: string } }
  | { RefList: { target_table: string } }
  | 'Attachment';

export interface Column {
  id: string;           // internal id e.g. "c_xxx"
  label: string;        // human readable
  col_type: ColType;
  position: number;
}

export interface Table {
  id: string;
  name: string;         // internal table name e.g. "t_xxx"
  display_name: string;
  columns: Column[];
}

export interface Row {
  row_id: string;
  [label: string]: string | undefined;   // keyed by column label for display
}

export interface GridData {
  columns: Array<{ id: string; label: string; col_type: string }>;
  rows: Row[];
}
