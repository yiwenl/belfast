from pathlib import Path

import pytest

from bridge import manifest, state


@pytest.fixture
def ws(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("WORKSPACE_ROOT", str(tmp_path))
    (tmp_path / "docs/reviews/queue").mkdir(parents=True)
    return tmp_path


def _save(slug: str, status: str):
    m = manifest.create_manifest(slug, "T", "S", [], [])
    m["status"] = status
    manifest.save_manifest(m)


def test_full_happy_path(ws: Path):
    _save("feat", "PENDING_REVIEW")
    state.transition("feat", "IN_REVIEW")
    state.transition("feat", "FEEDBACK_READY")
    state.transition("feat", "ACKNOWLEDGED")
    assert manifest.load_manifest("feat")["status"] == "ACKNOWLEDGED"
