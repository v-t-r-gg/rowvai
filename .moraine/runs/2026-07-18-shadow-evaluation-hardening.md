# Shadow evaluation final hardening

## Objective

Remove the candidate/outcome concurrency race, verify evaluation evidence before
scoring, strengthen fixture mutation checks, and remove the GitHub Actions
Node-runtime warning.

## Starting state

- Verified with ordinary Git commands from `/home/bone/Projects/Rowvai`.
- Repository root: `/home/bone/Projects/Rowvai`.
- Branch: `feat/shadow-evaluation-foundation` tracking
  `origin/feat/shadow-evaluation-foundation`.
- Local HEAD: `01ef77d2b41c8032cfd31194f6c634823bf11dc0`
  (`chore: add Moraine dogfood harness`), one commit ahead of remote HEAD
  `635a6807c11a245d457eef42a8e0d1089e9b6c25`.
- Initial worktree status: untracked `.moraine/`. The prompt's expected local HEAD
  was stale; no history was rewritten or discarded.

## Actions taken

- Read `AGENTS.md` and this active run record before substantive changes.
- Ran `git status --short --branch`, `git rev-parse --show-toplevel`,
  `git rev-parse HEAD`, `git remote -v`, and `git branch -vv`.
- Refactored candidate submission and outcome recording to acquire immediate
  SQLite transactions before loading mutable evaluation state.
- Added shared candidate-integrity verification and reused it for scoring and
  deterministic replay.
- Expanded fixture results with explicit operation, approval, record-revision,
  schema-revision, and evaluation-evidence mutation checks.
- Made actor/version report ordering deterministic so before/after evidence
  comparisons are stable for scenarios containing multiple agent versions.
- Updated GitHub Actions to `actions/checkout@v5` and
  `actions/setup-node@v5`, and added direct executable-fixture execution.
- Detected that local pre-task commit `01ef77d` contained the excluded
  `.gitignore` and `AGENTS.md` changes. Preserved it as a temporary patch,
  replayed only the hardening commit onto the remote PR head, and restored those
  files as unstaged local changes. They were not added to PR #2.
- Pushed hardening commit `86a1d4b5a6a703728810dd0dbbe8c5d8e12e5929`
  to `origin/feat/shadow-evaluation-foundation`.

## Decisions and rationale

- Candidate submission and outcome recording will both use SQLite
  `TransactionBehavior::Immediate`. Case reload, status checks, attempt
  eligibility, collection closure, candidate loading, integrity verification,
  scoring, and writes will occur under the same serialized write transaction.
  This provides a cross-process guarantee rather than an application mutex.
- A shared candidate-integrity verifier will be used by both scoring and replay.
  Any mismatch will abort with `evaluation_candidate_integrity_mismatch` rather
  than producing partial scoring evidence.
- Deterministic concurrency tests will use a test-only transaction-acquired hook
  with barriers. The production guarantee remains SQLite write serialization;
  the hook only controls test ordering without sleeps.

## Files changed

- `crates/rowva-store-sqlite/src/lib.rs`: serialized evaluation writes and
  shared evidence verification.
- `crates/rowva-store-sqlite/src/tests.rs`: deterministic two-connection race
  tests and corrupted-evidence rollback coverage.
- `crates/rowva-eval/src/main.rs`: explicit fixture mutation-safety evidence.
- `.github/workflows/ci.yml`: Node 24 action runtimes and direct fixture run.
- `.moraine/runs/2026-07-18-shadow-evaluation-hardening.md`: this run record.

## Tests and evidence

- `cargo fmt --all` completed successfully.
- `cargo test -p rowva-store-sqlite` passed: 22 tests, including
  `candidate_commit_before_outcome_is_included_in_atomic_scoring`,
  `outcome_commit_before_candidate_closes_collection_atomically`, and
  `corrupted_candidate_rolls_back_outcome_and_results`.
- The first executable fixture run exposed nondeterministic report ordering in
  the two-version scenario (9 passed, 1 failed). After sorting reports by actor
  ID and version, `cargo run -p rowva-eval -- fixture run
  fixtures/shadow/deal_stage_cases.json --json` passed all 10 scenarios. Every
  explicit candidate/replay mutation check reported `true`.
- `cargo fmt --all --check` passed.
- `cargo test --workspace` passed, including 22 SQLite tests and existing MCP,
  Tauri-adapter, core, and CLI coverage.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passed.
- `cargo test -p rowva-mcp --test stdio_smoke` passed (1 test).
- `cargo test -p rowva-eval` passed (2 CLI integration tests).
- `npm run build` passed. Node emitted the inherited local
  `module.register()` deprecation warning from the frontend toolchain.
- `npm run tauri build -- --no-bundle` built the frontend and compiled the
  release Tauri binary; the execution wrapper did not supply a final numeric
  exit-code field after compilation.
- `cargo tree --workspace --duplicates` completed successfully and reported
  inherited duplicate dependency versions in the Tauri/GTK graph.
- `cargo audit` was unavailable because the `cargo-audit` subcommand is not
  installed.
- `npm audit --audit-level=high` completed with exit 1 and reported inherited
  advisories: 5 total vulnerabilities (2 low, 1 moderate, 2 high), including
  high-severity findings in `picomatch` and `vite`. No broad dependency update
  was applied.
- GitHub Actions run `29637483655` passed in 1m44s. It ran formatting,
  workspace tests, Clippy, MCP smoke, CLI tests, the direct ten-scenario fixture,
  `npm ci`, and the frontend build. GitHub returned no check annotations, so the
  deprecated Node.js 20 action-runtime warning was absent.

## Risks and unresolved questions

- The local Moraine harness commit was not yet present on the remote branch at
  task start. It is being preserved as authoritative local history.
- The inherited npm audit advisories and local Node module deprecation remain
  outside this narrow concurrency/action-runtime pass.

## Outcome

Complete. Candidate collection and outcome scoring are serialized across SQLite
connections, candidate evidence is verified before scoring and replay, fixture
mutation evidence is explicit, and PR #2 CI uses current GitHub action runtimes.
The branch is ready for review; no merge was performed.
