# 0002: Semantic operations over arbitrary SQL

## Status

Accepted

## Context

Direct SQL bypasses validation, permissions, previews, revisions, attribution, and recoverable history, and couples callers to physical layout.

## Decision

All mutation adapters use versioned semantic commands. Commands declare capabilities and carry actor, reason, mode, concurrency, and idempotency context. No unrestricted SQL mutation tool will exist.

## Consequences

New behavior requires a deliberate command and tests. Storage can evolve without breaking callers. Read-only analytical querying may later use a separately sandboxed design.

## Alternatives considered

Parameterized SQL and table-level allowlists were rejected because neither expresses domain intent or safe undo. A generic patch API was deferred until its policy semantics are clear.

