from pathlib import Path

from fastapi import HTTPException

PROJECT_ROOT=Path.cwd().resolve()

IGNORE_DIRS = {
    ".git",
    ".venv",
    "__pycache__",
    "target",
    "node_modules",
    "data",
}

MAX_FILE_SIZE = 200_000

def resolve_project_path(path:str)->Path:
    target=(PROJECT_ROOT/path).resolve()

    try:
        target.relative_to(PROJECT_ROOT)
    except ValueError:
        raise HTTPException(status_code=400, detail="path is outside project")

    return target

def list_files()->list[str]:
    files=[]

    for path in PROJECT_ROOT.rglob("*"):
        if any(part in IGNORE_DIRS for part in path.parts):
            continue

        if path.is_file():
            files.append(path.relative_to(PROJECT_ROOT).as_posix())
    return files


def read_file(path:str)->dict[str,object]:
    target=resolve_project_path(path)

    if not target.exists():
        raise HTTPException(status_code=404,detail="file not found")
    
    if not target.is_file():
        raise HTTPException(status_code=400,detail="path is not a file")
    
    size=target.stat().st_size
    if size>MAX_FILE_SIZE:
        raise HTTPException(status_code=400,detail="file is too large")
    
    try:
        content=target.read_text(encoding="utf-8")
    except UnicodeEncodeError:
        raise HTTPException(status_code=400,detail="file is not utf-8 text")
    
    return {
        "path":target.relative_to(PROJECT_ROOT).as_posix(),
        "size":size,
        "content":content,
    }

