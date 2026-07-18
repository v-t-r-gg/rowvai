# RowvAI shadow evaluation

RowvAI is the public product name. Existing `rowva` internal namespaces and protocol identifiers are retained for compatibility.

Shadow mode is parallel recommendation, not preview, approval, or execution. RowvAI seals a minimal deal-stage case before a human outcome is exposed. External agents receive the same frozen bundle and submit attributable recommendations. Candidates never enter `_rowva_operations` or the approval inbox and have no commit identity.

## First workflow

`deal_stage_qualification_v1` freezes immutable object, record, and stage-field IDs; schema and record revisions; the configured stage field; selected record values; structured synthetic meeting evidence; creator; and SHA-256 digests. The digest detects substitution but is not a signature. Only an exact update of the frozen stage field is valid. No-change and abstain are explicit decisions. Evidence and reasons are untrusted data, bounded at import, stored as JSON, and never interpreted as instructions.

A human reference label is either linked to an existing committed human operation or recorded explicitly as no-change. The mutation is not replayed or duplicated. Recording an outcome closes blind collection. RowvAI can prove which bundle a candidate bound and what the evaluation interface exposed; it cannot prove an external agent did not inspect live state elsewhere.

## Replay versus reevaluation

Deterministic semantic replay reloads the sealed snapshot, runs the versioned frozen planner, and compares normalized field changes. It never reads the live record as its starting state and writes no operations, approvals, schema, or records. Model reevaluation is different: `case export` emits a versioned vendor-neutral bundle, and `candidate submit` imports a strict structured recommendation. RowvAI invokes no model and provides no orchestration.

## Scores and reports

Rubric revision 1 distinguishes exact change agreement, no-change agreement, false positives, false negatives, wrong stages, abstentions, and structured invalid proposals. Reports group by workflow, actor ID, and required actor version and return raw counts beside rates. Readiness revision 1 is conservative: fewer than 20 scored candidates is `insufficient_evidence`; error/invalid rates above 10% are `not_ready`; 90% agreement with sufficient evidence is only `candidate_for_human_review`. These are initial advisory heuristics, not statistical guarantees and never change policy, capabilities, approvals, or execution authority.

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

Committed fixtures are synthetic and network-free. Put private local dogfood inputs beneath `.local/eval/`, which Git ignores. Operators remain responsible for file permissions because meeting evidence, snapshots, reasons, and model output may contain sensitive data. Field-level redaction, configurable retention, a desktop dashboard, model invocation, additional workflows, and proof of external blindness are deferred.
