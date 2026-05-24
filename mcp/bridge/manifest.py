"""Queue manifest load/save and listing."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from bridge.paths import QUEUE_REL, feedback_path_for_slug, queue_manifest_path, validate_slug, workspace_root

VALID_STATUSES = frozenset(
    {"PENDING_REVIEW", "IN_REVIEW", "FEEDBACK_READY", "ACKNOWLEDGED"}
)

REQUIRED_FIELDS = (
    "slug",
    "status",
    "title",
    "summary",
    "submitted_at",
    "implementation_paths",
    "doc_paths",
    "feedback_path",
)


def _queue_dir() -> Path:
    d = workspace_root() / QUEUE_REL
    d.mkdir(parents=True, exist_ok=True)
    return d


def load_manifest(slug: str) -> dict[str, Any]:
    validate_slug(slug)
    path = queue_manifest_path(slug)
    if not path.is_file():
        raise FileNotFoundError(f"No queue manifest for slug '{slug}'.")
    with path.open(encoding="utf-8") as f:
        data = json.load(f)
    _validate_manifest(data, slug)
    return data


def save_manifest(data: dict[str, Any]) -> None:
    slug = data["slug"]
    _validate_manifest(data, slug)
    _queue_dir()
    path = queue_manifest_path(slug)
    with path.open("w", encoding="utf-8") as f:
        json.dump(data, f, indent=2)
        f.write("\n")


def _validate_manifest(data: dict[str, Any], expected_slug: str) -> None:
    for field in REQUIRED_FIELDS:
        if field not in data:
            raise ValueError(f"Manifest missing required field: {field}")
    if data["slug"] != expected_slug:
        raise ValueError("Manifest slug does not match filename.")
    if data["status"] not in VALID_STATUSES:
        raise ValueError(f"Invalid status: {data['status']}")


def list_manifests() -> list[dict[str, Any]]:
    qdir = _queue_dir()
    out: list[dict[str, Any]] = []
    for path in sorted(qdir.glob("*.json")):
        if path.name.startswith("_"):
            continue
        slug = path.stem
        try:
            out.append(load_manifest(slug))
        except (json.JSONDecodeError, ValueError, FileNotFoundError):
            continue
    return out


def create_manifest(
    slug: str,
    title: str,
    summary: str,
    implementation_paths: list[str],
    doc_paths: list[str],
) -> dict[str, Any]:
    validate_slug(slug)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    return {
        "slug": slug,
        "status": "PENDING_REVIEW",
        "title": title.strip(),
        "summary": summary.strip(),
        "submitted_at": now,
        "submitted_by": "cursor",
        "implementation_paths": implementation_paths,
        "doc_paths": doc_paths,
        "feedback_path": feedback_path_for_slug(slug),
        "handoff_message": None,
        "cursor_notes": None,
    }
