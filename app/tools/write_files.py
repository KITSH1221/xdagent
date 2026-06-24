from fastapi import HTTPException

from app.tools.read_files import (
    MAX_FILE_SIZE,
    ensure_protected_file,
    resolve_project_path,
)
from app.workspace import get_workspace_root
from app.tools.registry import tool

def validate_content_size(content:str)->None:
    size=len(content.encode("utf-8"))

    if size>MAX_FILE_SIZE:
        raise HTTPException(
            status_code=400,
            detail="the file is too large"
        )

@tool("create a new utf-8 file inside the project")
def write_file(path: str, content: str) -> dict[str, object]:
    target = resolve_project_path(path)
    project_root=get_workspace_root()
    ensure_protected_file(target)
    validate_content_size(content)

    if target.exists():
        raise HTTPException(status_code=400,detail="file has existed")

    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(content, encoding="utf-8")

    return {
        "path": target.relative_to(project_root).as_posix(),
        "size": target.stat().st_size,
        "create":True,
    }

@tool("replace exactly one occurrence of text in an existing project file")
def edit_file(path: str, old: str, new: str) -> dict[str, object]:
    project_root=get_workspace_root()
    target = resolve_project_path(path)
    ensure_protected_file(target)

    if not old:
        raise HTTPException(
            status_code=400,
            detail="old text cannot be empty",
        )

    if not target.exists():
        raise HTTPException(
            status_code=404,
            detail="file does not exist",
        )

    if not target.is_file():
        raise HTTPException(
            status_code=400,
            detail="path is not a file",
        )

    try:
        content = target.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        raise HTTPException(
            status_code=400,
            detail="file is not UTF-8 text",
        ) from exc

    occurrences = content.count(old)

    if occurrences == 0:
        raise HTTPException(
            status_code=400,
            detail="old text not found",
        )

    if occurrences > 1:
        raise HTTPException(
            status_code=409,
            detail=f"old text appears {occurrences} times",
        )

    updated = content.replace(old, new, 1)
    validate_content_size(updated)

    target.write_text(updated, encoding="utf-8")

    return {
        "path": target.relative_to(project_root).as_posix(),
        "replacements": 1,
        "size": target.stat().st_size,
    }
