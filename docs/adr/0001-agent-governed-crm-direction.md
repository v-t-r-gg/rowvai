# 0001: Agent-governed CRM direction

## Status

Accepted

## Context

The prototype centered a relational spreadsheet UI. Agents need stable semantics, attribution, safety, and observations independent of any UI.

## Decision

Rowva is a local-first, model-neutral CRM state and action runtime. A generic object/field/record engine supports optional CRM semantics. Humans and machines are explicit actors. The grid is one thin view.

## Consequences

Protocol stability and operation safety take priority over formulas and spreadsheet parity. UI and future adapters share one application service.

## Alternatives considered

Continuing a spreadsheet-first engine was rejected because labels and cells are an unstable machine protocol. Embedding an agent framework was rejected because reasoning belongs to external agents.

