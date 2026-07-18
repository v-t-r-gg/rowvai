# RowvAI shadow evaluation

RowvAI is the public product name. Existing `rowva` internal namespaces and protocol identifiers are retained for compatibility.

Shadow mode is parallel recommendation, not preview, approval, or execution. RowvAI seals a minimal deal-stage case before a human outcome is exposed. External agents receive the same frozen bundle and submit attributable recommendations. Candidates never enter `_rowva_operations` or the approval inbox and have no commit identity.

Dataset curation, terminal invalidation, immutable analysis runs, duplicate diagnostics, and confidence calibration are documented in [evaluation dataset quality](evaluation-data-quality.md). Live reports exclude invalidated cases by default; historical evidence remains inspectable.

## First workflow

`deal_stage_qualification_v1` freezes immutable object, record, and stage-field IDs; schema and record revisions; the complete configured stage field; selected record values; structured synthetic meeting evidence; and a permitted-output schema. Missing optional selected values are represented as JSON `null`. Only an exact, single-value `StageUpdateProposalV1` against the frozen target and revision is valid; the general operation `Command` protocol is not accepted. No-change and abstain are explicit decisions.

The exported object is `{ "bundle": EvaluationCaseBundleV1, "bundle_digest": "..." }`. The digest covers exactly `bundle`, including nested evidence, field configuration, and `permitted_output_schema`. To recompute it, recursively sort every JSON object key lexicographically, preserve array order and JSON scalar types, serialize compact UTF-8 JSON, then calculate lowercase hexadecimal SHA-256. The public Rust helpers are `canonical_json_bytes` and `evaluation_bundle_digest`. Export, candidate submission, and replay recompute stored integrity; replay also verifies the candidate-bound digest and candidate fingerprint. SHA-256 detects modification here—it is neither a signature nor proof of authorship.

Evidence, actor labels, versions, client/session IDs, and reasons are untrusted and byte-bounded at import. Imported capabilities are discarded before actor persistence and never convey authority. Selected field IDs are bounded, unique, and verified against the target object.

A human reference label is either linked to an existing committed human stage-only `UpdateRecord` operation or recorded explicitly as no-change. Outcome recording and later analysis use the same verifier. A linked operation must have an expected record revision, matching base schema revision, matching frozen before-stage, valid after-stage, and mutually consistent command, result, and change rows. The mutation is not replayed or duplicated. Recording an outcome closes blind collection. RowvAI can prove which bundle a candidate bound and what the evaluation interface exposed; it cannot prove an external agent did not inspect live state elsewhere.

Historical target IDs are snapshots, not foreign keys to mutable CRM rows. Deleting a live record therefore neither blocks normal CRM behavior nor cascades evaluation evidence; replay continues from the frozen snapshot.

## Replay versus reevaluation

Deterministic semantic replay reloads the sealed snapshot, runs the versioned frozen planner, and compares normalized field changes. It never reads the live record as its starting state and writes no operations, approvals, schema, or records. Model reevaluation is different: `case export` emits a versioned vendor-neutral bundle, and `candidate submit` imports a strict structured recommendation. RowvAI invokes no model and provides no orchestration.

## Scores and reports

At most one candidate per `(case_id, actor_id, actor_version)` is eligible for metrics. The first attempt is eligible; later retries are retained with attempt number and predecessor linkage but are ineligible, so retries cannot improve readiness. Different versions remain independently eligible.

Reports start from all candidate evidence, including pending and ineligible rows. They expose available cases, distinct attempted cases, submitted attempts, eligible decisions, ineligible retries, pending eligible decisions, scored eligible decisions, valid/invalid scored decisions, exclusions, and each verdict. `agreement_count = exact_change_agreements + no_change_agreements`. Accuracy uses non-abstaining eligible scored decisions; abstentions reduce coverage and are excluded from accuracy. Coverage is non-abstaining divided by eligible scored; invalid-proposal rate is invalid scored divided by eligible scored. A zero denominator produces `null`, not an invented rate.

Readiness revision 1 uses those same definitions. Fewer than 20 eligible scored decisions is `insufficient_evidence`; coverage below 80% is `not_ready`; error/invalid outcomes above 10% of non-abstaining decisions are `not_ready`; and at least 90% combined agreement can yield only `candidate_for_human_review`. Exactly 10% error remains within the threshold. These are initial advisory heuristics, not statistical guarantees and never change policy, capabilities, approvals, or execution authority.

## Developer workflow

```bash
cargo run -p rowva-eval -- fixture run fixtures/shadow/deal_stage_cases.json
cargo run -p rowva-eval -- case create --workspace /absolute/demo.rowva --input case.json
cargo run -p rowva-eval -- case export --workspace /absolute/demo.rowva --case-id evc_...
cargo run -p rowva-eval -- candidate submit --workspace /absolute/demo.rowva --input candidate.json
cargo run -p rowva-eval -- outcome record --workspace /absolute/demo.rowva --input outcome.json
cargo run -p rowva-eval -- replay --workspace /absolute/demo.rowva --case-id evc_... --candidate-id evn_...
cargo run -p rowva-eval -- report --workspace /absolute/demo.rowva --json
```

`fixture run` creates a temporary workspace per scenario, constructs schema and data, verifies export integrity, submits candidates, records the human reference, scores, replays, compares expected verdicts, and checks that shadow actions did not mutate authoritative state. Any mismatch exits nonzero. Committed fixtures are synthetic and network-free.

`fixture quality-run fixtures/quality/deal_stage_quality_cases.json --json` executes the complementary sixteen-scenario dataset-quality suite, including canonical analysis idempotency, lifecycle invalidation, calibration, duplicate/repeated evidence diagnostics, structural metadata privacy, explicit sensitive export, and authority-mutation checks.

Put private local dogfood inputs beneath `.local/eval/`, which Git ignores. Operators remain responsible for file permissions because meeting evidence, snapshots, reasons, and model output may contain sensitive data. Evaluation evidence is retained append-only and can contain values; field-level redaction, configurable retention, a desktop dashboard, model invocation, additional workflows, and proof of external blindness are deferred.
