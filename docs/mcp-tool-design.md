# MCP tool design: implemented surface

The stdio server uses official `rmcp` 2.2.0 and negotiates the SDK's stable supported MCP versions. All input structs reject unknown fields. Tools never accept workspace paths, actors, capabilities, SQL, or replacement commit commands. Full examples and operation workflow are in [mcp.md](mcp.md).

| Tool | Input | Output | Capability | Annotation / safety |
|---|---|---|---|---|
| `rowva_workspace_describe` | `{}` | format/schema version, object count, fixed actor/capabilities/features | `workspace.read` | read-only, idempotent, closed-world |
| `rowva_schema_describe` | `{object_id?}` | logical objects and complete safe fields | `schema.read` | read-only, idempotent, closed-world |
| `rowva_records_search` | `{object_id,filters?,limit?,cursor?}` | bounded field-ID records and cursor | `records.read` | read-only, idempotent, closed-world |
| `rowva_record_get` | `{object_id,record_id}` | record, revision, timestamps, schema revision | `records.read` | read-only, idempotent, closed-world |
| `rowva_operation_preview` | `{command,reason?,idempotency_key?,correlation?}` | durable proposal, diff, witnesses, policy, fingerprint | command capability | non-destructive proposal write, idempotent with key |
| `rowva_operation_commit` | `{operation_id,preview_fingerprint}` | versioned durable receipt | command capability rechecked | state-changing, non-delete, idempotent |
| `rowva_operation_get` | `{operation_id}` | own-actor operation, changes/error/correlation | `operations.read` | read-only, idempotent |
| `rowva_operation_list` | safe filters, cursor, bounded limit | own-actor concise summaries | `operations.read` | read-only, idempotent |

Preview mutation union contains only `create_record` and `update_record`. Update requires `expected_revision`; create receives its immutable record ID during normalization so preview and commit refer to the same target. Delete and schema mutation are absent.

Search operators are `equals`, `not_equals`, `contains`, `is_null`, and `is_not_null`. SQLite receives only bound parameters. Page size is 1–100, filters cap at 10, and ordering/cursors use immutable record IDs.

The preview fingerprint is SHA-256 of recursively key-sorted JSON containing preview version, workspace/operation/actor identities, normalized command, reason, idempotency key, schema revision, state witnesses, and required capability. It is not a signature. Stored input—not MCP input—is applied at commit.

Implemented transitions:

```text
proposed → committed
proposed → conflicted
proposed → failed
```

The model reserves rejected, expired, awaiting-approval, reverted. MCP cannot self-approve or commit a non-proposed operation. Operation visibility is currently own-actor-only as an explicit adapter policy.

No `execute_sql`, direct create/update commit, arbitrary command runner, file resource, prompt, or schema tool exists.
