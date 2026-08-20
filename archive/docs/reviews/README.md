# Feature reviews (Antigravity)

Per-feature architectural reviews from Antigravity. Library-wide notes remain in [`docs/feedback.md`](../feedback.md).

| Path                                | Purpose                                                                |
| ----------------------------------- | ---------------------------------------------------------------------- |
| `queue/<slug>.json`                 | Handoff manifest from Cursor (`PENDING_REVIEW` → … → `ACKNOWLEDGED`)   |
| `queue/<slug>-implementation.md`    | Implementation summary for review (source of truth for what was built) |
| `<slug>-antigravity.md`             | Review output from Antigravity                                         |
| `_template-antigravity-feedback.md` | Copy when writing feedback                                             |
| `queue/_template-handoff.json`      | Copy when submitting from Cursor                                       |

Workflow: [`docs/workflows/cursor-antigravity-review.md`](../workflows/cursor-antigravity-review.md)  
MCP tools: [`mcp/README.md`](../../mcp/README.md)
