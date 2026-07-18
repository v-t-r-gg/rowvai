import { invoke } from '@tauri-apps/api/core';
import type { ApprovalRequest, Field, GridData, ObjectSummary, OperationPreview, OperationReceipt, OperationRecord } from '../types/rowva';

export type { Field, FieldKind, GridData, ObjectSummary, RecordData, RowvaError } from '../types/rowva';

export const commands = {
  newWorkbook: (name: string) =>
    invoke<string>('new_workbook', { name }),

  createTable: (displayName: string) =>
    invoke<string>('create_table', { displayName }),

  addColumn: (tableId: string, label: string) =>
    invoke<Field>('add_column', { tableId, label }),

  getGridData: (tableId: string) =>
    invoke<GridData>('get_grid_data', { tableId }),

  insertRow: (tableId: string, values: Record<string, unknown>) =>
    invoke<string>('insert_row', { tableId, values }),

  openWorkbook: (path: string) =>
    invoke<string>('open_workbook', { path }),

  getTables: () =>
    invoke<ObjectSummary[]>('get_tables'),

  updateCell: (tableId: string, rowId: string, colId: string, value: unknown, expectedRevision: number) =>
    invoke('update_cell', { tableId, rowId, colId, value, expectedRevision }),

  deleteRow: (tableId: string, rowId: string, expectedRevision: number) =>
    invoke('delete_row_cmd', { tableId, rowId, expectedRevision }),
  listApprovals: () => invoke<ApprovalRequest[]>('list_approval_inbox', { limit: 100 }),
  getApproval: (approvalRequestId:string) => invoke<ApprovalRequest>('get_approval_detail',{approvalRequestId}),
  approve: (approvalRequestId:string,proposalFingerprint:string,reason?:string) => invoke<OperationReceipt>('approve_and_execute_operation',{approvalRequestId,proposalFingerprint,reason}),
  reject: (approvalRequestId:string,reason?:string) => invoke<ApprovalRequest>('reject_operation',{approvalRequestId,reason}),
  revise: (approvalRequestId:string,replacementValues:Record<string,unknown>,reason?:string) => invoke<{original_operation_id:string;successor:OperationPreview}>('revise_operation',{approvalRequestId,replacementValues,reason}),
  previewUndo: (operationId:string,reason?:string) => invoke<OperationPreview>('preview_operation_undo',{operationId,reason}),
  commitUndo: (operationId:string,previewFingerprint:string) => invoke<OperationReceipt>('commit_operation_undo',{operationId,previewFingerprint}),
  listOperations: () => invoke<OperationRecord[]>('list_operation_history',{limit:100}),
};
