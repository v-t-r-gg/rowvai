# RowvAI MCP stdio server

`rowva-mcp` exposes one local RowvAI workspace as a governed MCP tool server. It supports discovery, bounded reads, durable mutation previews, exact preview commits, and operation inspection. Agent record mutations require human approval by default. It does not expose SQLite, filesystem paths, schema mutation, delete, approval actions, undo, HTTP, prompts, resources, or an agent runtime.

## Build and start

```bash
cargo build --release -p rowva-mcp
./target/release/rowva-mcp serve \
  --workspace /absolute/path/to/sales.rowva \
  --actor-id agent_example --actor-name "Example Agent" \
  --actor-type agent --actor-version 1.0 \
  --capability workspace.read --capability schema.read \
  --capability records.read --capability records.create \
  --capability records.update --capability operations.read
```

The workspace must exist and is canonicalized once at startup. Tool inputs cannot select a workspace. Actor ID, name, and capabilities are immutable for the process. Actor IDs without `act_` are normalized with that prefix. Supported actor types are `agent` and `integration`. Delete is never granted implicitly.

CLI values take precedence over `ROWVA_WORKSPACE`, `ROWVA_ACTOR_ID`, `ROWVA_ACTOR_NAME`, `ROWVA_ACTOR_TYPE`, `ROWVA_ACTOR_VERSION`, and comma-separated `ROWVA_CAPABILITIES`. There is no configuration file in this release, avoiding an ambiguous additional authority source.

Generic client configuration (syntax varies by client):

```json
{"mcpServers":{"rowva":{"command":"/absolute/path/to/rowva-mcp","args":["serve","--workspace","/absolute/path/to/sales.rowva","--actor-id","agent_example","--actor-name","Example Agent","--actor-version","1.0","--capability","workspace.read","--capability","schema.read","--capability","records.read","--capability","records.create","--capability","records.update","--capability","operations.read"]}}}
```

Protect client configuration because launch arguments reveal local paths and authority.

## Tools and workflow

- `rowva_workspace_describe`: safe process/workspace metadata.
- `rowva_schema_describe`: all objects or one immutable object ID.
- `rowva_records_search`: up to 100 records, 10 typed filters, deterministic cursor paging.
- `rowva_record_get`: one field-ID-keyed record and revision.
- `rowva_operation_preview`: durable create/update proposal; no business-state write.
- `rowva_operation_commit`: exact stored proposal ID plus fingerprint only; pending human review returns `approval_required`.
- `rowva_operation_get`: one operation owned by the configured actor.
- `rowva_operation_list`: bounded summaries owned by the configured actor.

Discover workspace/schema, find Acme, and retrieve its deal. Then preview:

```json
{"command":{"type":"update_record","object_id":"obj_deals","record_id":"rec_acme_deal","expected_revision":4,"values":{"fld_stage":"Qualified"}},"reason":"Customer confirmed qualification in meeting mtg_123","idempotency_key":"mtg_123_qualification","correlation":{"source_type":"meeting","source_id":"mtg_123"}}
```

Review changes, policy, and witnesses. Commit only with the returned identity:

```json
{"operation_id":"op_123","preview_fingerprint":"4e0c..."}
```

The versioned receipt contains actor, revisions, affected records, exact changes, correlation, and commit time. Retrieve it with `rowva_operation_get`, then confirm revision 5 with `rowva_record_get`.

The SHA-256 fingerprint binds canonical command JSON, operation/workspace/actor identities, reason, idempotency key, schema revision, state witnesses, required capability, and preview format. It is an integrity hash, not a signature. Commit reloads trusted stored input and never accepts replacement mutation data. Schema or record drift marks the proposal `conflicted` and requires a new preview.

Agent previews enter `awaiting_approval` and include `approval_request_id`, `expires_at`, and `commit_allowed: false`. A distinct trusted human decides through the desktop inbox. Allowed transitions include rejection, expiration, conflict, superseding, execution, and governed revert. MCP cannot self-approve.

## Limits, errors, and logging

Search/history default to 25 and cap at 100; filters cap at 10; field changes at 100; reason and serialized correlation at 2,048 bytes. The SDK frames stdio messages, but configurable request-byte/depth and result-byte caps are deferred. Operators are `equals`, `not_equals`, `contains`, `is_null`, and `is_not_null`.

Expected failures are structured MCP tool errors, for example `{"error":{"code":"record_revision_conflict","category":"conflict","retryable":true,"recovery":{"action":"preview_again"}}}`. Raw SQLite errors, paths, tables, and Rust types are never returned.

Stdout is MCP JSON-RPC only. Diagnostics go to stderr. `ROWVA_LOG` controls filtering and `ROWVA_LOG_FORMAT=json` selects JSON. Events carry safe IDs/status, never full values. The durable ledger—not telemetry—is authoritative. Startup exits nonzero for invalid path/workspace/actor/capability or open/migration/locking failure. Legacy 0.1 files require export and recreation.
