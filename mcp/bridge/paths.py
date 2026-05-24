"""Workspace root and safe repo-relative path resolution."""

from __future__ import annotations

import os
import re
from pathlib import Path

SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")

QUEUE_REL = Path("docs/reviews/queue")
REVIEWS_REL = Path("docs/reviews")


def workspace_root() -> Path:
    raw = os.environ.get("WORKSPACE_ROOT", "").strip()
    if raw:
        return Path(raw).resolve()
    # server lives at mcp/server.py → repo root is parent of mcp/
    mcp_dir = Path(__file__).resolve().parent.parent
    if mcp_dir.name == "mcp" and (mcp_dir.parent / "packages").is_dir():
        return mcp_dir.parent
    return Path.cwd().resolve()


def validate_slug(slug: str) -> None:
    if not SLUG_RE.match(slug):
        raise ValueError(
            f"Invalid slug '{slug}': use lowercase letters, digits, and hyphens only."
        )


def normalize_repo_path(path: str) -> str:
    p = path.strip().replace("\\", "/")
    if not p:
        raise ValueError("Path must not be empty.")
    if p.startswith("/") or ".." in p.split("/"):
        raise ValueError(f"Path must be repo-relative with no '..': {path}")
    return p


def resolve_repo_path(path: str, *, must_exist: bool = False) -> Path:
    rel = normalize_repo_path(path)
    root = workspace_root()
    full = (root / rel).resolve()
    if not str(full).startswith(str(root)):
        raise ValueError(f"Path escapes workspace: {path}")
    if must_exist and not full.is_file():
        raise ValueError(f"File not found: {path}")
    return full


def queue_manifest_path(slug: str) -> Path:
    validate_slug(slug)
    return workspace_root() / QUEUE_REL / f"{slug}.json"


def feedback_path_for_slug(slug: str) -> str:
    validate_slug(slug)
    return f"docs/reviews/{slug}-antigravity.md"


def feedback_file_path(slug: str) -> Path:
    return workspace_root() / feedback_path_for_slug(slug)
