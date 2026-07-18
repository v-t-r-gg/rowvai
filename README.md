# RowvAI

RowvAI is the public product name. Existing `rowva` internal namespaces and protocol identifiers are retained for compatibility.

RowvAI is an early local-first, model-neutral CRM runtime for humans and autonomous agents. It stores portable workspaces in SQLite and exposes stable semantic operations with actor attribution, previews, revisions, idempotency, structured errors, and an append-only audit trail. The desktop grid is a thin human control plane, not the product boundary.

## Current foundation

- Tauri-independent `rowva-core` protocol and application contract
- typed workspace/object/field/record/actor/operation IDs
- durable tagged field configuration and field-ID-keyed record values
- versioned transactional SQLite migrations
- atomic semantic object, field, record create/update/delete operations
- actor/capability policy boundary, preview, operation/change log
- schema and record revisions plus optimistic concurrency
- committed-mutation idempotency
- thin Tauri compatibility commands and React grid
- official-SDK MCP stdio server with bounded reads, durable previews, exact commits, receipts, and history
- developer `rowva-eval` CLI with frozen deal-stage shadow cases, strict candidate import, deterministic replay, scoring, and advisory actor-version reports

The desktop includes an Agent Activity inbox for governed changes. Shadow evaluation remains developer-facing and cannot alter authority. See [shadow evaluation](docs/shadow-evaluation.md), [approvals](docs/approvals.md), [undo](docs/undo.md), [MCP usage](docs/mcp.md), [architecture](docs/architecture.md), and the [threat model](docs/security/mcp-threat-model.md).

## Development

```bash
npm install
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
npm run build
npm run tauri dev
./target/debug/rowva-mcp --help
```

The unversioned 0.1 prototype format is not auto-imported because it omitted configuration needed for faithful recovery. Preserve old files and export visible values with the old build before recreating them. Migration compatibility begins with versioned format 1.

Licensed under MIT or Apache-2.0.
