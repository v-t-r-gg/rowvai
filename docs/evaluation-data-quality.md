# Evaluation dataset quality

RowvAI evaluation remains observational. Invalidation, datasets, analysis, calibration, warnings, and readiness never grant capabilities, alter policy, approve work, or execute CRM changes.

## Lifecycle and reproducibility

A human or system actor may append one terminal invalidation to a case. The record binds the actor, category, bounded reason, prior status, bundle digest, related duplicate case when supplied, and UTC time. Candidates, outcomes, scorer results, and frozen bundles remain intact. Invalidated cases reject later candidates and outcomes and are excluded from current reports by default.

An evaluation dataset is an immutable snapshot, not a saved query. Dataset protocol revision 1 supports explicit case IDs or a bounded all-scored time selection. Creation resolves and persists an ordered list of scored, integrity-valid, non-invalidated cases. The canonical SHA-256 digest covers the complete manifest, including membership and selection provenance. SQLite triggers reject update and delete of datasets and membership.

An analysis run binds the dataset digest, actor ID/version, scorer revision, readiness-rubric revision, quality revision, calibration revision, current invalidation state, and verified candidate/result evidence. Exact repeated input returns the existing run. Changed evidence or invalidation produces a new run. Reports are stored as canonical JSON with a digest and cannot be updated or deleted. Digests detect modification; they are not signatures or proof of authorship.

Scorer and readiness revisions are explicitly dispatched. Revision 1 is currently the only supported revision; unknown revisions fail rather than silently selecting the newest code. Existing scorer rows are verified against deterministic recomputation, missing revision-1 rows are appended, and no scorer evidence is overwritten.

## Quality revision 1

Diagnostics include lifecycle inclusion/exclusion counts, human reference-label balance, destination/transition counts, candidate coverage, abstentions, invalid proposals, retries, and confidence availability. Zero-denominator rates are `null`.

Duplicate content identity revision 1 excludes case ID, creator, workspace ID, and creation time. It includes the workflow, frozen field definition, revisions, minimal snapshot, current stage, meeting input, and permitted output schema. Reports also group repeated meeting-evidence digests, source meeting IDs, and target-record/base-revision pairs. These hashes identify repeated bytes, not semantic equivalence, and never auto-invalidate evidence.

Initial warnings use explicit thresholds: fewer than 20 included cases or calibrated samples is insufficient evidence. Existing readiness revision 1 retains its documented 20-case, 80% coverage, 10% error, and 90% agreement thresholds. Quality warnings do not modify readiness classes.

## Confidence calibration revision 1

Confidence is optional, finite, and within `[0, 1]`. It means the agent's stated probability that its non-abstaining recommendation exactly agrees with the eventual human operational reference. It is not safety, objective truth, approval probability, or authority.

Calibration includes eligible first attempts that are included, scored, semantically valid, non-abstaining, and have confidence. It excludes and counts missing confidence, abstentions, invalid proposals, retries, invalidated cases, and unscored cases. Correctness is `exact_agreement` or `no_change_agreement`.

The Brier score is `mean((confidence - correctness)^2)`. ECE uses ten fixed bins: `[0.0,0.1)`, …, `[0.9,1.0]`; it is the sample-weighted mean absolute difference between mean confidence and empirical agreement in each nonempty bin. Empty metrics are `null`. Fewer than 20 calibrated samples receives `insufficient_calibration_evidence`; ECE remains sensitive to binning and sample size.

## Export and privacy

`metadata` is the default profile and is marked `metadata_only`. It omits frozen values, meeting evidence, stage values, reasons, display names, sessions, client metadata, and operation payloads. It retains opaque IDs, digests, statuses, actor IDs/versions, verdicts, confidence, timestamps, and aggregate diagnostics.

`full_local` is explicit, file-only in the CLI, marked `contains_sensitive_values: true`, and creates a new file with mode `0600` on Unix. It contains frozen bundles and may contain customer values, meeting evidence, and reasons. Contents are never logged. Each export wraps the exact payload with its canonical digest.

SQLite still retains value-bearing frozen evidence. Field-level stored redaction and automatic retention deletion are deferred because rewriting sealed bundles would invalidate reproducibility. Local dogfood inputs and full exports must be protected. RowvAI cannot prove that an external model was blind to live workspace information.

There is one workflow (`deal_stage_qualification_v1`), no desktop evaluation UI, no model invocation, and no automatic earned autonomy.

## CLI

```text
rowva-eval case list --workspace WORKSPACE
rowva-eval case invalidate --workspace WORKSPACE --input invalidation.json
rowva-eval dataset create --workspace WORKSPACE --input dataset.json
rowva-eval dataset list --workspace WORKSPACE
rowva-eval dataset show --workspace WORKSPACE --dataset-id evd_...
rowva-eval dataset quality --workspace WORKSPACE --dataset-id evd_... --actor-id act_... --actor-version 1.0
rowva-eval dataset export --workspace WORKSPACE --dataset-id evd_... --profile metadata
rowva-eval dataset export --workspace WORKSPACE --dataset-id evd_... --profile full_local --output protected.json
rowva-eval analysis run --workspace WORKSPACE --input analysis.json
rowva-eval analysis list --workspace WORKSPACE
rowva-eval analysis show --workspace WORKSPACE --analysis-id eva_...
```

JSON input types deny unknown fields. Human-readable output is the default and global `--json` produces machine-readable output, except that `full_local` always requires a new output file and never writes sensitive payloads to stdout.
