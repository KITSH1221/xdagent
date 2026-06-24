"""Per-agent-run workspace binding for project file tools."""

from contextlib import contextmanager
from contextvars import ContextVar
from pathlib import Path
from typing import Iterator


_current_workspace: ContextVar[Path | None] = ContextVar(
    "current_workspace",
    default=None,
)


def get_workspace_root() -> Path:
    workspace = _current_workspace.get()

    if workspace is None:
        raise RuntimeError("workspace is not bound")

    return workspace


@contextmanager
def bind_workspace(path: str) -> Iterator[Path]:
    workspace = Path(path).expanduser().resolve()

    if not workspace.exists():
        raise ValueError(
            f"workspace does not exist: {workspace}"
        )

    if not workspace.is_dir():
        raise ValueError(
            f"workspace is not a directory: {workspace}"
        )

    token = _current_workspace.set(workspace)

    try:
        yield workspace
    finally:
        # FIX: restore the previous request context to prevent path leakage.
        _current_workspace.reset(token)
