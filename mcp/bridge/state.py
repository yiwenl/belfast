"""Status transition guards."""

from __future__ import annotations

from typing import Any

from bridge.manifest import load_manifest, save_manifest

TRANSITIONS: dict[str, frozenset[str]] = {
    "PENDING_REVIEW": frozenset({"IN_REVIEW", "FEEDBACK_READY"}),
    "IN_REVIEW": frozenset({"FEEDBACK_READY", "PENDING_REVIEW"}),
    "FEEDBACK_READY": frozenset({"ACKNOWLEDGED"}),
    "ACKNOWLEDGED": frozenset(),
}


class StateError(ValueError):
    """Invalid status transition."""


def transition(slug: str, new_status: str) -> dict[str, Any]:
    manifest = load_manifest(slug)
    current = manifest["status"]
    allowed = TRANSITIONS.get(current, frozenset())
    if new_status not in allowed:
        raise StateError(
            f"Cannot transition '{slug}' from {current} to {new_status}. "
            f"Allowed: {', '.join(sorted(allowed)) or 'none'}."
        )
    manifest["status"] = new_status
    save_manifest(manifest)
    return manifest


def set_field(slug: str, **fields: Any) -> dict[str, Any]:
    manifest = load_manifest(slug)
    manifest.update(fields)
    save_manifest(manifest)
    return manifest
