import json
from pathlib import Path

import pytest

from bridge import manifest, state
from bridge.paths import queue_manifest_path


@pytest.fixture
def ws(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("WORKSPACE_ROOT", str(tmp_path))
    q = tmp_path / "docs/reviews/queue"
    q.mkdir(parents=True)
    src = tmp_path / "packages/belfast/src/a.ts"
    src.parent.mkdir(parents=True)
    src.write_text("export {}")
    doc = tmp_path / "docs/api/A.md"
    doc.parent.mkdir(parents=True)
    doc.write_text("# A")
    return tmp_path


def test_create_and_load(ws: Path):
    m = manifest.create_manifest(
        "feature-a",
        "Title",
        "Summary here.",
        ["packages/belfast/src/a.ts"],
        ["docs/api/A.md"],
    )
    manifest.save_manifest(m)
    loaded = manifest.load_manifest("feature-a")
    assert loaded["status"] == "PENDING_REVIEW"
    assert loaded["slug"] == "feature-a"


def test_transition_guards(ws: Path):
    m = manifest.create_manifest("x", "T", "S", [], [])
    m["implementation_paths"] = []
    m["doc_paths"] = []
    manifest.save_manifest(m)
    state.transition("x", "IN_REVIEW")
    with pytest.raises(state.StateError):
        state.transition("x", "ACKNOWLEDGED")
