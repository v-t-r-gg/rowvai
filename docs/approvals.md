# Human approvals

Approval is durable evidence over one exact fingerprinted proposal, not a mutable boolean. After capability checks, agent and integration record creates and updates require human review by default. Local-human mutations remain explicit `allow` decisions.

The lifecycle is `awaiting_approval` to `executed`, `rejected`, `expired`, `superseded`, `execution_conflicted`, or `execution_failed`. Revisions create a successor operation and fingerprint while preserving and linking the original. Proposals expire after 24 hours and are marked lazily; audit evidence is retained.

Approve-and-execute requires a distinct human with `operations.approve`, verifies the exact fingerprint, then reuses the trusted stored-command commit path and its schema/record witnesses. The immutable decision commits before execution, so a later conflict is auditable. MCP exposes no approval tool and cannot submit identity, capabilities, or approval claims.

The Agent Activity desktop view shows proposer, reason, policy reason, timestamps, and field diffs. Stored content is untrusted text. Current limitations are a fixed policy revision, fixed expiration, local-human assignment, and no field-level audit redaction.
