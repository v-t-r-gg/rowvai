# 0003: Local SQLite storage and versioned migrations

## Status

Accepted

## Context

Portable ownership requires a durable local file. The prototype had ad-hoc schema creation and incomplete field metadata.

## Decision

Use SQLite current-state tables plus append-only operation/change logs. Use normalized record values keyed by immutable IDs. Apply embedded, ordered migrations transactionally and record version, application version, UTC time, and identifier.

## Consequences

Atomic changes and portable files are straightforward. JSON values require application validation and may need indexes later. The unversioned 0.1 format remains read-only legacy and is not auto-imported because it cannot preserve missing configuration; users must export visible prototype data and recreate it. Compatibility guarantees start at migration 1.

## Alternatives considered

Dynamic physical tables were rejected for migration and observability complexity. Event sourcing was rejected as disproportionate. A cloud database conflicts with local-first ownership.

