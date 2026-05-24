import os
from pathlib import Path

import pytest

from bridge.paths import normalize_repo_path, resolve_repo_path, validate_slug


def test_validate_slug():
    validate_slug("draw-depth")
    with pytest.raises(ValueError):
        validate_slug("Bad_Slug")


def test_reject_traversal(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("WORKSPACE_ROOT", str(tmp_path))
    (tmp_path / "ok.txt").write_text("x")
    with pytest.raises(ValueError):
        normalize_repo_path("../etc/passwd")
    p = resolve_repo_path("ok.txt")
    assert p.name == "ok.txt"


def test_must_exist(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setenv("WORKSPACE_ROOT", str(tmp_path))
    with pytest.raises(ValueError):
        resolve_repo_path("missing.txt", must_exist=True)
