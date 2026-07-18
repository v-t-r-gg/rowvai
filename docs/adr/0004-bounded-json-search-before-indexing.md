# 0004: Bounded JSON search before indexing

## Status

Accepted

## Context

Normalized JSON values preserve field identity but scalar filters may slow as objects grow. Evidence for one permanent indexing strategy is not available.

## Decision

Use parameterized SQLite filtering with deterministic ID pagination, five safe operators, ten filters, and a 100-record maximum. Do not accept SQL or materialize the whole workspace in Rust.

## Consequences

Behavior is safe and measurable, but filters may scan value rows. Capture query plans and benchmarks before selecting generated scalar indexes, projections, JSON indexes, or FTS.

## Alternatives considered

Dynamic user indexes and universal FTS were premature. Loading all records in Rust was rejected for memory and pagination behavior.
