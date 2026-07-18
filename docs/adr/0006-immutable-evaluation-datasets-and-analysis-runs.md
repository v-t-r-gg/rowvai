# 0006: Immutable evaluation datasets and analysis runs

## Status

Accepted

## Context

Individual shadow cases and results were durable, but a live query could not prove which evidence produced a report. Later invalidation, retries, scorer changes, or sampling imbalance could make an analysis irreproducible or misleading.

## Decision

RowvAI stores terminal append-only invalidations, canonical immutable dataset manifests with resolved membership, and immutable analysis runs. Analysis verifies case, candidate, outcome, and scorer evidence; explicitly dispatches revisions; records exclusions; computes quality and confidence calibration; and binds its inputs and report with SHA-256 digests. Metadata-first export separates reviewable evidence from value-bearing local bundles. Evaluation has no dependency or call path into authority policy.

## Consequences

Historical analyses remain reproducible after later invalidation. Dataset membership cannot drift with a filter. Storage grows because evidence is not deleted or rewritten. Hashes detect byte changes but provide neither signatures nor semantic equivalence. Initial warning and calibration thresholds are heuristics requiring human interpretation.

## Alternatives considered

- Mutable saved queries: rejected because membership would drift.
- Recomputing reports in place: rejected because historical claims would change.
- Automatic duplicate invalidation: rejected because digest equality is not a quality judgment.
- Exporting full evidence by default: rejected because frozen CRM and meeting data may be sensitive.
- Feeding readiness into policy: rejected because evaluation is advisory evidence, not authority.
