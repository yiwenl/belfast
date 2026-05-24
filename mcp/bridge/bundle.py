"""Review bundle: manifest + file contents with size cap."""

from __future__ import annotations

from typing import Any

from bridge.paths import resolve_repo_path

MAX_BUNDLE_BYTES = 200 * 1024


def build_review_bundle(manifest: dict[str, Any]) -> dict[str, Any]:
    paths: list[str] = list(manifest.get("implementation_paths", [])) + list(
        manifest.get("doc_paths", [])
    )
    files: list[dict[str, str]] = []
    warnings: list[str] = []
    total = 0
    truncated = False

    for rel in paths:
        try:
            full = resolve_repo_path(rel, must_exist=False)
            if not full.is_file():
                warnings.append(f"Missing file (skipped): {rel}")
                continue
            content = full.read_text(encoding="utf-8", errors="replace")
            encoded = content.encode("utf-8")
            if total + len(encoded) > MAX_BUNDLE_BYTES:
                remaining = MAX_BUNDLE_BYTES - total
                if remaining > 0:
                    content = encoded[:remaining].decode("utf-8", errors="replace")
                    content += "\n\n[... truncated ...]"
                else:
                    warnings.append(f"Omitted (bundle full): {rel}")
                    truncated = True
                    continue
                truncated = True
            total += len(content.encode("utf-8"))
            files.append({"path": rel, "content": content})
        except ValueError as e:
            warnings.append(str(e))

    return {
        "manifest": manifest,
        "files": files,
        "truncated": truncated,
        "warnings": warnings,
    }


def format_bundle_text(bundle: dict[str, Any]) -> str:
    lines = [
        f"# Review bundle: {bundle['manifest']['slug']}",
        f"**Title:** {bundle['manifest']['title']}",
        f"**Summary:** {bundle['manifest']['summary']}",
        "",
    ]
    if bundle["warnings"]:
        lines.append("## Warnings")
        for w in bundle["warnings"]:
            lines.append(f"- {w}")
        lines.append("")
    if bundle["truncated"]:
        lines.append("_Bundle was truncated at 200KB. Read remaining paths from disk._")
        lines.append("")
    for item in bundle["files"]:
        lines.append(f"## File: `{item['path']}`")
        lines.append("")
        lines.append("```")
        lines.append(item["content"])
        lines.append("```")
        lines.append("")
    return "\n".join(lines)
