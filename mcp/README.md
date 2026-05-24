# Belfast review bridge (MCP v2)

MCP server coordinating **Cursor** (build + submit) and **Antigravity** (review) for Belfast features. Uses the same files as the v1 workflow:

- Queue: `docs/reviews/queue/<slug>.json`
- Feedback: `docs/reviews/<slug>-antigravity.md`

Workflow overview: [`docs/workflows/cursor-antigravity-review.md`](../docs/workflows/cursor-antigravity-review.md)  
Implementation plan: [`docs/workflows/mcp-review-bridge-v2-plan.md`](../docs/workflows/mcp-review-bridge-v2-plan.md)

## Install

Requires [uv](https://github.com/astral-sh/uv) (or Python 3.10+ with pip).

```bash
cd mcp
uv sync --extra dev
```

Run tests:

```bash
uv run pytest -q
```

## Cursor setup

1. Ensure [`.cursor/mcp.json`](../.cursor/mcp.json) exists (committed in repo).
2. **Cursor Settings → MCP** → enable **belfast-review-bridge**.
3. Reload the window if tools do not appear.
4. Optional: set `WORKSPACE_ROOT` in `mcp.json` `env` if the server cannot find the repo (monorepo / unusual cwd).

The server auto-detects the repo root as the parent of `mcp/` when `packages/` exists there.

### Without uv

```json
{
  "mcpServers": {
    "belfast-review-bridge": {
      "command": "python3",
      "args": ["mcp/server.py"],
      "cwd": "/absolute/path/to/belfast"
    }
  }
}
```

## Antigravity setup

Edit `~/.gemini/antigravity/mcp_config.json` ([docs](https://antigravity.google/docs/mcp)):

```json
{
  "mcpServers": {
    "belfast-review-bridge": {
      "command": "uv",
      "args": ["run", "--directory", "/absolute/path/to/belfast/mcp", "server.py"],
      "cwd": "/absolute/path/to/belfast",
      "env": { "WORKSPACE_ROOT": "/absolute/path/to/belfast" }
    }
  }
}
```

Restart Antigravity after saving.

## MCP tools

### Cursor

| Tool                        | Description                              |
| --------------------------- | ---------------------------------------- |
| `submit_feature_for_review` | Create queue manifest (`PENDING_REVIEW`) |
| `get_review_status`         | Status for one slug or all               |
| `get_feedback_for_cursor`   | Read feedback when `FEEDBACK_READY`      |
| `acknowledge_feedback`      | Set `ACKNOWLEDGED`                       |

### Antigravity

| Tool                     | Description                                |
| ------------------------ | ------------------------------------------ |
| `list_pending_reviews`   | Slugs with `PENDING_REVIEW` or `IN_REVIEW` |
| `claim_review`           | `PENDING_REVIEW` → `IN_REVIEW`             |
| `get_review_bundle`      | Manifest + file contents (~200KB cap)      |
| `submit_review_feedback` | Write feedback markdown, `FEEDBACK_READY`  |
| `notify_cursor_handoff`  | Store message for user to return to Cursor |

## Status flow

`PENDING_REVIEW` → `IN_REVIEW` → `FEEDBACK_READY` → `ACKNOWLEDGED`

(`PENDING_REVIEW` may skip directly to `FEEDBACK_READY` if review is submitted without claim.)

## Troubleshooting

| Issue                    | Fix                                                    |
| ------------------------ | ------------------------------------------------------ |
| Tools missing in Cursor  | Enable server in MCP settings; run `cd mcp && uv sync` |
| Path not found on submit | Paths must be repo-relative and exist                  |
| Feedback not ready       | Antigravity must call `submit_review_feedback` first   |
| Wrong repo root          | Set `WORKSPACE_ROOT` to absolute Belfast path          |
| v1 fallback              | Disable MCP; edit queue JSON manually per workflow doc |

## v1 without MCP

If the server is off, use the file-only steps in [`docs/workflows/cursor-antigravity-review.md`](../docs/workflows/cursor-antigravity-review.md) (v1 section).
