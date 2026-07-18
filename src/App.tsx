import { useEffect, useState } from 'react';
import { commands } from './lib/tauri';
import Grid from './components/Grid';
import type { GridData, ObjectSummary, RowvaError } from './types/rowva';
import type { OperationRecord } from './types/rowva';
import AgentActivity from './components/AgentActivity';

function App() {
  const [workbookPath, setWorkbookPath] = useState<string | null>(null);
  const [tables, setTables] = useState<ObjectSummary[]>([]);
  const [currentTableId, setCurrentTableId] = useState<string | null>(null);
  const [gridData, setGridData] = useState<GridData>({ columns: [], rows: [] });
  const [status, setStatus] = useState<string>('');
  const [view,setView]=useState<'data'|'activity'|'history'>('data');
  const [history,setHistory]=useState<OperationRecord[]>([]);
  useEffect(()=>{if(view==='history')void commands.listOperations().then(setHistory).catch(()=>setStatus('Unable to load operation history'))},[view]);

  const handleNewWorkbook = async () => {
    const name = prompt('Workbook name (e.g. My CRM)') || 'My CRM';
    try {
      const path = await commands.newWorkbook(name);
      setWorkbookPath(path);
      setStatus('✓ Workbook created!');
      setTimeout(() => setStatus(''), 3000);
    } catch (e) {
      const error = e as RowvaError;
      setStatus(`Error [${error.code ?? 'unknown'}]: ${error.message ?? String(e)}`);
    }
  };

  const handleNewTable = async () => {
  if (!workbookPath) return;
  const name = prompt('Table name (e.g. Contacts)') || 'Contacts';
  try {
    const tableId = await commands.createTable(name);
    const newTable = { id: tableId, display_name: name };
    setTables([...tables, newTable]);
    setCurrentTableId(tableId);
    refreshGrid(tableId);
    console.log('✅ Table created:', tableId);
  } catch (e: any) {
    console.error('Create table error:', e);
    alert('Failed to create table: ' + (e?.message || e || 'Unknown error'));
  }
};

const handleAddColumn = async () => {
  if (!currentTableId) return;
  const label = prompt('Column name (e.g. Name)') || 'New Column';
  try {
    await commands.addColumn(currentTableId, label);
    refreshGrid(currentTableId);
    console.log('✅ Column added:', label);
  } catch (e: any) {
    console.error('Add column error:', e);
    alert('Failed to add column: ' + (e?.message || e || 'Unknown error'));
  }
};

const refreshGrid = async (tableId: string) => {
  try {
    const data = await commands.getGridData(tableId);
    setGridData(data);
  } catch (e: any) {
    console.error('Grid refresh error:', e);
  }
};

const handleAddRow = async () => {
  if (!currentTableId) return;
  try {
    await commands.insertRow(currentTableId, {});
    await refreshGrid(currentTableId);
    console.log('✅ Row added');
  } catch (e: any) {
    console.error('Add row error:', e);
    alert('Failed to add row: ' + (e?.message || e || 'Unknown error'));
  }
};
  return (
    <div className="min-h-screen bg-zinc-950 text-white font-mono">
      {/* Header - matches your screenshot */}
      <div className="border-b border-zinc-800 p-6">
        <div className="max-w-6xl mx-auto">
          <h1 className="text-5xl font-bold tracking-tighter">Rowva</h1>
          <p className="text-zinc-400 mt-1">Local-first Relational Spreadsheet CRM</p>
        </div>
      </div>

      <div className="max-w-6xl mx-auto p-6">
        {!workbookPath ? (
          <div className="flex gap-4 items-center">
            <input
              type="text"
              defaultValue="My CRM"
              className="bg-zinc-900 border border-zinc-700 rounded px-4 py-3 text-lg w-80 focus:outline-none focus:border-emerald-500"
              id="wb-name"
            />
            <button
              onClick={handleNewWorkbook}
              className="bg-white hover:bg-emerald-500 hover:text-white text-black px-8 py-3 rounded font-medium transition"
            >
              Create New Workbook (.rowva)
            </button>
          </div>
        ) : (
          <>
            <div className="flex items-center gap-4 mb-8 bg-zinc-900 border border-emerald-500/30 rounded p-4">
              <div className="text-emerald-500 text-2xl">✓</div>
              <div>
                <p className="font-medium">Workbook ready</p>
                <p className="text-xs text-zinc-500 break-all">{workbookPath}</p>
              </div>
            </div>

            <nav className="flex gap-2 mb-6" aria-label="Workspace views">{(['data','activity','history'] as const).map(item=><button key={item} onClick={()=>setView(item)} className={`px-4 py-2 rounded capitalize ${view===item?'bg-emerald-600':'bg-zinc-800'}`}>{item==='activity'?'Agent Activity':item==='history'?'Operation History':'Data'}</button>)}</nav>

            {view==='activity'?<AgentActivity onCommitted={()=>currentTableId&&void refreshGrid(currentTableId)}/>:view==='history'?<div className="border border-zinc-800 rounded bg-zinc-900"><h2 className="text-xl p-4 border-b border-zinc-800">Operation history</h2>{history.map(op=><div key={op.id} className="p-4 border-b border-zinc-800"><div className="flex justify-between"><span className="capitalize">{op.kind.replaceAll('_',' ')}</span><span>{op.status}</span></div><div className="text-xs text-zinc-500">{op.actor.display_name} · {new Date(op.created_at).toLocaleString()} · {op.id}</div>{op.reason&&<p className="text-sm mt-1">{op.reason}</p>}</div>)}</div>:<div className="flex gap-8">
              {/* Sidebar - Tables (Grist-style) */}
              <div className="w-64 bg-zinc-900 border border-zinc-800 rounded p-4 h-fit">
                <div className="flex justify-between items-center mb-4">
                  <h2 className="text-lg font-medium">Tables</h2>
                  <button
                    onClick={handleNewTable}
                    className="bg-emerald-600 hover:bg-emerald-500 px-3 py-1 rounded text-sm"
                  >
                    + New
                  </button>
                </div>
                {tables.length === 0 && <p className="text-zinc-500 text-sm">Create your first table</p>}
                <div className="space-y-1">
                  {tables.map(t => (
                    <button
                      key={t.id}
                      onClick={() => { setCurrentTableId(t.id); refreshGrid(t.id); }}
                      className={`w-full text-left px-4 py-2 rounded text-sm hover:bg-zinc-800 transition ${currentTableId === t.id ? 'bg-zinc-800' : ''}`}
                    >
                      {t.display_name}
                    </button>
                  ))}
                </div>
              </div>

              {/* Main Grid Area */}
              <div className="flex-1">
                {currentTableId && (
                  <>
                    <div className="flex justify-between items-center mb-4">
                      <h3 className="text-2xl font-medium">
                        {tables.find(t => t.id === currentTableId)?.display_name}
                      </h3>
                      <div className="flex gap-2">
                        <button
                          onClick={handleAddColumn}
                          className="bg-zinc-800 hover:bg-white hover:text-black border border-zinc-700 px-5 py-2 rounded text-sm transition"
                        >
                          + Add Column
                        </button>
                        <button
                          onClick={handleAddRow}
                          className="bg-emerald-600 hover:bg-emerald-500 px-5 py-2 rounded text-sm transition"
                        >
                          + Add Row
                        </button>
                      </div>
                    </div>
                    <div className="border border-zinc-800 rounded overflow-hidden">
                      <Grid 
                        data={gridData} 
                        onDataChange={() => refreshGrid(currentTableId)} 
                        tableId={currentTableId} 
                      />
                    </div>
                  </>
                )}
              </div>
            </div>
            }
          </>
        )}

        {status && <div className="fixed bottom-4 right-4 bg-emerald-600 text-white px-6 py-3 rounded">{status}</div>}
      </div>
    </div>
  );
}

export default App;
