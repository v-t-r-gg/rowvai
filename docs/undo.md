# Governed undo

Undo is a new inverse semantic operation, never an out-of-band rewrite. It currently supports only committed, unreverted `update_record` operations.

The record revision must equal the original receipt's post-commit revision and every affected value must still equal the original `after` value. RowvAI previews an inverse update with the exact `before` values and current revision witness. The inverse has its own ID and fingerprint and links through `reverts_operation_id`.

Normal preview commit applies the inverse. Only after that transaction succeeds is the original marked `reverted` and linked through `reverted_by_operation_id`. Drift returns `undo_state_conflict`; there is no force or partial undo. Creates, deletes, schema changes, failures, and external side effects are not undoable.
