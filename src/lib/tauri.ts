import { invoke } from '@tauri-apps/api/core';
import type { GridData } from '../types/rowva';

export type { Table, GridData, Row, Column, ColType } from '../types/rowva';

export const commands = {
  newWorkbook: (name: string) =>
    invoke<string>('new_workbook', { name }),

  createTable: (displayName: string) =>
    invoke<string>('create_table', { displayName }),

  addColumn: (tableId: string, label: string) =>
    invoke('add_column', { tableId, label }),

  getGridData: (tableId: string) =>
    invoke<GridData>('get_grid_data', { tableId }),

  insertRow: (tableId: string, values: Record<string, string>) =>
    invoke<string>('insert_row', { tableId, values }),

  openWorkbook: (path: string) =>
    invoke<string>('open_workbook', { path }),

  getTables: () =>
    invoke<any[]>('get_tables'),

  updateCell: (tableId: string, rowId: string, colId: string, value: string) =>
    invoke('update_cell', { tableId, rowId, colId, value }),

  deleteRow: (tableId: string, rowId: string) =>
    invoke('delete_row_cmd', { tableId, rowId }),
};