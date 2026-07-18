# 0005: Shadow evaluation before earned autonomy

## Status

Accepted

## Context

Approval evidence shows whether a human authorized a proposal, but does not provide an unbiased comparison of agent judgment. Evaluating only changed records omits false positives. Increasing authority without frozen, versioned evidence would couple heuristics to safety policy prematurely.

## Decision

Add append-only, frozen deal-stage evaluation cases with explicit change and no-change human reference labels. Shadow candidates are non-executable evidence outside operations and approvals. Deterministic replay uses a frozen semantic planner; external model reevaluation uses strict export/import. Versioned scoring and readiness reports are advisory and have no dependency path to capability or policy decisions.

## Consequences

Agent versions can be compared on identical inputs, invalid recommendations remain visible, and rates retain honest denominators. SQLite retains additional value-bearing evidence. The first planner is deliberately workflow-specific, the human label is operational rather than objectively infallible, and external information leakage cannot be proven.

## Alternatives considered

- Reuse operation previews: rejected because previews are executable proposals and enter approval policy.
- Score only committed stage changes: rejected because it cannot measure false positives.
- Invoke models inside RowvAI: rejected to preserve model neutrality and deterministic boundaries.
- Automatically grant authority from metrics: rejected because evaluation and authorization must remain independent.
