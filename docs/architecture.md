# RowvAI architecture

## Baseline before this transition

The 0.1 prototype was one Tauri crate plus React. Tauri commands opened SQLite connections and contained validation, label-to-column translation, state mutation, and raw SQL orchestration. Objects used dynamically created physical tables. Reads returned values keyed by mutable labels. There were no migrations, tests, operation log, durable workspace identity, revisions, actor context, policy boundary, previews, or idempotency. Opening a file invented a new in-memory workbook ID. Formula and relation configuration was loaded as empty placeholders. The README correctly called this an early prototype, but its “full persistence” and formula-oriented dependency claims were ahead of implemented behavior.

## Current boundaries

```text
React grid / rowva-eval CLI / rowva-mcp stdio
                  |
          transport adapters
                  |
       rowva-core application protocol
                  |
       rowva-store-sqlite application
                  |
      versioned portable SQLite file
```

`rowva-core` contains typed identifiers, actors, capabilities, field definitions, commands, operation envelopes/results, structured errors, the policy boundary, and the synchronous `RowvaApplication` contract. It has no Tauri, SQLite, CLI, browser, HTTP, or MCP dependency.

`rowva-store-sqlite` implements that contract. It owns connection configuration, migrations, validation, transactions, current-state repositories, idempotency, revisions, and append-only observations. It does not expose a connection. The initial implementation keeps application orchestration close to SQLite so transaction ownership is explicit; repository modules can be extracted when their boundaries become real rather than speculative.

`src-tauri` is a compatibility adapter. Commands deserialize old table/column/row-shaped UI calls, establish the `act_local_user` human actor, invoke the application service, and serialize results. No parallel business implementation remains. The grid now receives field-ID-keyed records.

`rowva-mcp` binds one canonical workspace and immutable process actor. Eight strict handlers translate MCP input into the same `RowvaApplication`; the official `rmcp` SDK owns JSON-RPC and stdio framing. It issues no SQL and logs only to stderr.

`rowva-eval` is a developer adapter over the transport-neutral `EvaluationApplication`. Migration 5 stores immutable minimal cases, independent candidate attempts, explicit human outcomes, and versioned results. The typed exported bundle—including its permitted-output schema—is the canonical SHA-256 input. Candidate imports are narrow stage proposals; their authority-bearing capabilities are discarded. One attempt per case/actor/version is metrics-eligible, while retries remain evidence. Evaluation is intentionally disconnected from operation commit, approval, policy, and capability code. Frozen replay verifies both bundle and candidate integrity and uses captured schema/record evidence rather than authoritative live state.

Migration 6 adds append-only invalidations, atomically built/sealed dataset manifests, and immutable analysis runs. Relational membership must exactly equal the ordered canonical manifest and cannot change after sealing. Analysis verifies cases, candidates/retries, human outcomes and linked operation evidence, and scorer revision 1; records exact inclusion/exclusion; computes versioned quality and calibration diagnostics; and persists canonical evidence/input/report digests. SQLite triggers protect candidate, outcome, result, invalidation, frozen-case, dataset, membership, and analysis evidence. Metadata export structurally omits value-bearing bundles; explicit full-local export remains file-only in the CLI. None of these services depend on or modify authorization policy.

## Storage and migrations

The current format also uses `_rowva_approval_requests` and append-only `_rowva_approval_decisions`. Migration 4 adds approvals, expiration, successor, and revert links plus inbox/history indexes. Values live in operation requests and change records; approval rows reference that evidence rather than duplicating it. Foreign keys are enabled and busy timeout is five seconds.

Embedded migrations have increasing integer versions and stable identifiers. Opening a workspace creates the migration ledger, checks each version, and applies missing migrations in individual transactions. A ledger row includes the crate application version and UTC timestamp. Migration tests cover order and reopen/idempotency. Evaluation target IDs in migration 5 are historical snapshot identifiers rather than foreign keys to live objects, fields, or records, so CRM deletion cannot cascade evidence or be blocked by it; foreign keys remain within the evaluation evidence graph. Migration 6 preserves that evidence graph without cascading deletes and enforces dataset and analysis immutability with triggers.

The unversioned 0.1 development format is not auto-imported. It cannot faithfully represent formula/relation configuration and attempting inference risks silent loss. Old files remain untouched. Development users should keep a copy and manually export visible values from the old build before recreating the workspace. Compatibility guarantees begin with migration version 1; subsequent format changes require ordered, tested migrations.

## Operations and safety

Every accepted command declares a required capability and passes the central policy before storage. The desktop policy is permissive by giving the local actor all current capabilities. Preview and commit share the same command and response shape. Preview runs full validation and proposed writes inside a transaction that is rolled back.

Commit inserts current state, actor, operation result, before/after changes, and revision updates in one transaction. Failed commits that reach execution are recorded separately with a structured error after rollback. `(actor_id, idempotency_key)` is unique: exact request replay returns the prior result, while different payload reuse conflicts. Record updates and deletes accept an expected revision. Workspace schema revision increments for object and field changes.

Durable previews store normalized trusted input and exact changes. SHA-256 binds workspace, operation, actor, command, reason, idempotency, capability, schema revision, and record witnesses. Agent record mutations atomically create a pending approval. A human decision is committed before the shared commit path runs; its execution outcome is then recorded. Revision creates a linked successor. Governed undo creates a linked inverse update and marks the original reverted only after success. The log is not event sourcing: current-state tables remain authoritative.

## Deferred intentionally

Configurable/assigned approval policy, signed receipts, field-level redaction, retention compaction, granular actor credentials, relation behavior, bulk commands, backups, legacy import, CRM templates, formulas, attachments, sync, HTTP, and embedded models are deferred.
