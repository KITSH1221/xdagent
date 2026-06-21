from fastapi import HTTPException

from app.tools.read_files import PROJECT_ROOT, resolve_project_path
from app.schemas import WriteFileRequest,EditFileRequest

PROTECTED_FILES = {
    ".env",
}


def ensure_not_protected(target) -> str:
    relative_path = target.relative_to(PROJECT_ROOT).as_posix()

    if relative_path in PROTECTED_FILES:
        raise HTTPException(status_code=400, detail="the file is protected")

    return relative_path


def write_file(path: str, content: str) -> dict[str, object]:
    target = resolve_project_path(path)
    relative_path = ensure_not_protected(target)

    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")

    return {
        "path": relative_path,
        "size": target.stat().st_size,
    }


def edit_file(path: str, old: str, new: str) -> dict[str, object]:
    target = resolve_project_path(path)
    relative_path = ensure_not_protected(target)

    if not target.exists():
        raise HTTPException(status_code=404, detail="file not exists")

    content = target.read_text(encoding="utf-8")

    if old not in content:
        raise HTTPException(status_code=400, detail="old text not found")

    updated = content.replace(old, new, 1)
    target.write_text(updated, encoding="utf-8")

    return {
        "path": relative_path,
        "replacements": 1,
    }
