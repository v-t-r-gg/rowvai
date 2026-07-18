export type FieldKind =
  | { type: 'text'; config: { max_length: number | null } }
  | { type: 'number'; config: { precision: number | null } }
  | { type: 'boolean' | 'date' | 'date_time' | 'attachment' }
  | { type: string; config?: unknown };

export interface Field { id: string; object_id: string; display_name: string; key: string; kind: FieldKind; required: boolean; unique: boolean }
export interface ObjectSummary { id: string; display_name: string }
export interface RecordData { record_id: string; revision: number; values: Record<string, unknown> }
export interface GridData { columns: Array<{ id: string; label: string; col_type: FieldKind }>; rows: RecordData[] }
export interface RowvaError { category: string; code: string; message?: string; details?: unknown }
export type OperationStatus = 'proposed'|'awaiting_approval'|'committed'|'rejected'|'failed'|'reverted'|'conflicted'|'expired'|'superseded';
export interface Actor { id:string; actor_type:'human'|'agent'|'integration'|'system'; display_name:string; actor_version?:string|null }
export interface FieldChange { object_id:string; record_id?:string|null; field_id?:string|null; before?:unknown; after?:unknown }
export interface OperationRecord { id:string; actor:Actor; kind:string; status:OperationStatus; reason?:string|null; created_at:string; committed_at?:string|null; changes:FieldChange[]; preview_fingerprint?:string|null; result?:unknown; reverts_operation_id?:string|null; reverted_by_operation_id?:string|null }
export type ApprovalStatus='pending'|'approved'|'rejected'|'expired'|'superseded'|'executed'|'execution_conflicted'|'execution_failed';
export interface ApprovalDecision { id:string; decided_by:Actor; decision:'approve'|'reject'; reason?:string|null; decided_at:string }
export interface ApprovalRequest { id:string; operation_id:string; proposal_fingerprint:string; status:ApprovalStatus; requested_by:Actor; policy_reason_codes:string[]; requested_at:string; expires_at:string; operation:OperationRecord; decision?:ApprovalDecision|null }
export interface OperationPreview { operation_id:string; status:OperationStatus; preview_fingerprint:string; changes:FieldChange[]; policy_decision:'allow'|'deny'|'require_approval'; approval_request_id?:string|null; expires_at?:string|null }
export interface OperationReceipt { operation_id:string; status:OperationStatus; affected_records:Array<{object_id:string;record_id:string;before_revision?:number|null;after_revision:number}>; changes:FieldChange[]; committed_at:string; replayed:boolean }
