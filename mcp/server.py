#!/usr/bin/env python3
"""Belfast review bridge — MCP server for Cursor ↔ Antigravity handoffs."""

from __future__ import annotations

import json
import sys
import traceback
from datetime import datetime, timezone
from typing import Any

from mcp.server.fastmcp import FastMCP

from bridge import bundle, manifest, state
from bridge.paths import (
    feedback_file_path,
    normalize_repo_path,
    resolve_repo_path,
    validate_slug,
)
from bridge.bundle import build_review_bundle, format_bundle_text

mcp = FastMCP("belfast-review-bridge")

PENDING_STATUSES = frozenset({"PENDING_REVIEW", "IN_REVIEW"})


def _err(message: str) -> str:
    print(message, file=sys.stderr)
    return f"Error: {message}"


def _validate_paths(paths: list[str], *, must_exist: bool) -> list[str]:
    normalized: list[str] = []
    for p in paths:
        normalize_repo_path(p)
        if must_exist:
            resolve_repo_path(p, must_exist=True)
        normalized.append(normalize_repo_path(p))
    return normalized


def _age_hours(iso: str) -> float:
    try:
        dt = datetime.fromisoformat(iso.replace("Z", "+00:00"))
        delta = datetime.now(timezone.utc) - dt
        return round(delta.total_seconds() / 3600, 1)
    except ValueError:
        return -1


# --- Cursor tools ---


@mcp.tool()
def submit_feature_for_review(
    slug: str,
    title: str,
    summary: str,
    implementation_paths: list[str],
    doc_paths: list[str],
) -> str:
    """
    Submit a Belfast feature for Antigravity review.
    Creates docs/reviews/queue/<slug>.json with status PENDING_REVIEW.
    """
    try:
        validate_slug(slug)
        impl = _validate_paths(implementation_paths, must_exist=True)
        docs = _validate_paths(doc_paths, must_exist=True)
        if queue := _existing_pending(slug):
            return _err(
                f"Manifest already exists for '{slug}' with status {queue['status']}."
            )
        data = manifest.create_manifest(slug, title, summary, impl, docs)
        manifest.save_manifest(data)
        return (
            f"Submitted '{slug}' for Antigravity review (PENDING_REVIEW).\n\n"
            f"Feedback will be written to: {data['feedback_path']}\n\n"
            "Next: open Antigravity on the same branch, enable belfast-review-bridge MCP, "
            "and run list_pending_reviews → claim_review → get_review_bundle."
        )
    except Exception as e:
        return _err(str(e))


def _existing_pending(slug: str) -> dict[str, Any] | None:
    try:
        m = manifest.load_manifest(slug)
        if m["status"] != "ACKNOWLEDGED":
            return m
    except FileNotFoundError:
        pass
    return None


@mcp.tool()
def get_review_status(slug: str = "") -> str:
    """Return review status for one slug or all queue entries."""
    try:
        if slug:
            validate_slug(slug)
            m = manifest.load_manifest(slug)
            age = _age_hours(m["submitted_at"])
            return json.dumps(
                {**m, "age_hours": age},
                indent=2,
            )
        entries = manifest.list_manifests()
        if not entries:
            return "No reviews in queue."
        lines = ["| slug | status | age (h) | title |", "| --- | --- | --- | --- |"]
        for m in entries:
            lines.append(
                f"| {m['slug']} | {m['status']} | {_age_hours(m['submitted_at'])} | {m['title']} |"
            )
        return "\n".join(lines)
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def get_feedback_for_cursor(slug: str) -> str:
    """Return Antigravity feedback when status is FEEDBACK_READY."""
    try:
        validate_slug(slug)
        m = manifest.load_manifest(slug)
        if m["status"] != "FEEDBACK_READY":
            return _err(
                f"Feedback not ready for '{slug}' (status: {m['status']}). "
                "Wait for Antigravity to submit_review_feedback."
            )
        fb_path = feedback_file_path(slug)
        if not fb_path.is_file():
            return _err(f"Feedback file missing: {m['feedback_path']}")
        text = fb_path.read_text(encoding="utf-8")
        handoff = m.get("handoff_message") or ""
        parts = [f"## Feedback for `{slug}`\n", text]
        if handoff:
            parts.append(f"\n---\n**Handoff:** {handoff}")
        return "\n".join(parts)
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def acknowledge_feedback(slug: str, cursor_notes: str = "") -> str:
    """Mark review ACKNOWLEDGED after Cursor and user discuss feedback."""
    try:
        validate_slug(slug)
        state.transition(slug, "ACKNOWLEDGED")
        if cursor_notes.strip():
            state.set_field(slug, cursor_notes=cursor_notes.strip())
        return f"Acknowledged review for '{slug}'."
    except state.StateError as e:
        return _err(str(e))
    except Exception as e:
        return _err(str(e))


# --- Antigravity tools ---


@mcp.tool()
def list_pending_reviews() -> str:
    """List slugs awaiting or in progress for Antigravity review."""
    try:
        pending = [
            m
            for m in manifest.list_manifests()
            if m["status"] in PENDING_STATUSES
        ]
        if not pending:
            return "No pending reviews."
        lines = ["Pending reviews:", ""]
        for m in pending:
            lines.append(
                f"- **{m['slug']}** ({m['status']}, {_age_hours(m['submitted_at'])}h): {m['title']}"
            )
        return "\n".join(lines)
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def claim_review(slug: str) -> str:
    """Claim a review: PENDING_REVIEW → IN_REVIEW."""
    try:
        validate_slug(slug)
        m = manifest.load_manifest(slug)
        if m["status"] == "IN_REVIEW":
            return f"Already IN_REVIEW: '{slug}'."
        if m["status"] != "PENDING_REVIEW":
            return _err(f"Cannot claim '{slug}' with status {m['status']}.")
        state.transition(slug, "IN_REVIEW")
        return f"Claimed review for '{slug}'. Use get_review_bundle next."
    except state.StateError as e:
        return _err(str(e))
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def get_review_bundle(slug: str) -> str:
    """Return manifest and source/doc file contents for review (max ~200KB)."""
    try:
        validate_slug(slug)
        m = manifest.load_manifest(slug)
        if m["status"] not in PENDING_STATUSES | {"FEEDBACK_READY"}:
            return _err(
                f"Bundle not available for '{slug}' with status {m['status']}."
            )
        b = build_review_bundle(m)
        return format_bundle_text(b)
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def submit_review_feedback(slug: str, feedback_markdown: str) -> str:
    """
    Write docs/reviews/<slug>-antigravity.md and set status FEEDBACK_READY.
    """
    try:
        validate_slug(slug)
        body = feedback_markdown.strip()
        if not body:
            return _err("feedback_markdown must not be empty.")
        m = manifest.load_manifest(slug)
        if m["status"] not in ("IN_REVIEW", "PENDING_REVIEW"):
            return _err(
                f"Cannot submit feedback for '{slug}' with status {m['status']}."
            )
        fb_path = feedback_file_path(slug)
        fb_path.parent.mkdir(parents=True, exist_ok=True)
        if not body.lstrip().startswith("#"):
            title = m.get("title", slug)
            body = (
                f"# Antigravity review: {title}\n\n"
                f"**Slug:** `{slug}`  \n"
                f"**Manifest:** `docs/reviews/queue/{slug}.json`\n\n"
                f"{body}"
            )
        fb_path.write_text(body + "\n", encoding="utf-8")
        if m["status"] != "FEEDBACK_READY":
            state.transition(slug, "FEEDBACK_READY")
        return (
            f"Feedback saved to {m['feedback_path']} (FEEDBACK_READY).\n"
            "Call notify_cursor_handoff, then ask the user to switch to Cursor."
        )
    except state.StateError as e:
        return _err(str(e))
    except Exception as e:
        return _err(str(e))


@mcp.tool()
def notify_cursor_handoff(slug: str, message: str) -> str:
    """Store a handoff message for the user to continue in Cursor."""
    try:
        validate_slug(slug)
        m = manifest.load_manifest(slug)
        if m["status"] != "FEEDBACK_READY":
            return _err(
                f"Handoff requires FEEDBACK_READY (current: {m['status']}). "
                "Call submit_review_feedback first."
            )
        msg = message.strip() or (
            f'Antigravity review complete for "{slug}". '
            f"In Cursor, ask: get_feedback_for_cursor slug={slug}"
        )
        state.set_field(slug, handoff_message=msg)
        return (
            f"Handoff recorded for '{slug}'.\n\n"
            f"**Tell the user:** {msg}"
        )
    except Exception as e:
        return _err(str(e))


if __name__ == "__main__":
    try:
        mcp.run()
    except Exception:
        traceback.print_exc(file=sys.stderr)
        raise
