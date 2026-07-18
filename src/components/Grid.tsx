import { useState } from 'react';
import { createColumnHelper, flexRender, getCoreRowModel, useReactTable } from '@tanstack/react-table';
import { commands } from '../lib/tauri';
import type { GridData, RecordData } from '../types/rowva';

type Props = { 
  data: GridData; 
  onDataChange: () => void; 
  tableId?: string;
};

interface EditingCell {
  rowId: string;
  colId: string;
  initialValue: string;
}

export default function Grid({ data, onDataChange, tableId }: Props) {
  const [editing, setEditing] = useState<EditingCell | null>(null);
  const [editValue, setEditValue] = useState('');

  const columnHelper = createColumnHelper<RecordData>();

  const columns = data.columns.map((col: any) =>
    columnHelper.accessor(row => row.values[col.id], {
      id: col.id,
      header: col.label,
      cell: (info) => {
        const rowId = info.row.original.record_id;
        const revision = info.row.original.revision;
        const colId = col.id;
        const value = info.getValue() || '';

        const isEditingThis = editing?.rowId === rowId && editing?.colId === colId;

        if (isEditingThis) {
          return (
            <input
              autoFocus
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={() => commitEdit(rowId, colId, revision)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  commitEdit(rowId, colId, revision);
                } else if (e.key === 'Escape') {
                  cancelEdit();
                }
              }}
              className="w-full px-3 py-1 bg-zinc-950 border border-emerald-500 text-white text-sm focus:outline-none"
            />
          );
        }

        return (
          <div
            className="px-4 py-2 cursor-text hover:bg-zinc-800 min-h-[34px]"
            onDoubleClick={() => startEdit(rowId, colId, String(value))}
          >
            {value == null || value === '' ? '—' : String(value)}
          </div>
        );
      },
    })
  );

  const table = useReactTable({
    data: data.rows,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  function startEdit(rowId: string, colId: string, initialValue: string) {
    setEditing({ rowId, colId, initialValue });
    setEditValue(initialValue);
  }

  async function commitEdit(rowId: string, colId: string, expectedRevision: number) {
    if (!editing || !tableId) return;
    const newValue = editValue;

    try {
      await commands.updateCell(tableId, rowId, colId, newValue, expectedRevision);
      setEditing(null);
      setEditValue('');
      onDataChange(); // refresh grid
    } catch (e: any) {
      console.error('Failed to update cell', e);
      alert('Failed to save: ' + (e?.message || e));
      cancelEdit();
    }
  }

  function cancelEdit() {
    setEditing(null);
    setEditValue('');
  }

  return (
    <div className="overflow-auto max-h-[calc(100vh-280px)]">
      <table className="min-w-full border-collapse">
        <thead className="bg-zinc-900 sticky top-0">
          {table.getHeaderGroups().map(headerGroup => (
            <tr key={headerGroup.id}>
              {headerGroup.headers.map(header => (
                <th key={header.id} className="px-4 py-3 text-left border-b border-zinc-800 font-medium">
                  {flexRender(header.column.columnDef.header, header.getContext())}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody>
          {table.getRowModel().rows.map(row => (
            <tr key={row.id} className="hover:bg-zinc-900 border-b border-zinc-800">
              {row.getVisibleCells().map(cell => (
                <td key={cell.id} className="border-r border-zinc-800 last:border-r-0">
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {data.rows.length === 0 && (
        <div className="p-12 text-center text-zinc-500">No rows yet. Use "+ Add Row" above.</div>
      )}
    </div>
  );
}
