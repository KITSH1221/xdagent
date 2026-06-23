from pathlib import Path

from fastapi import HTTPException
from app.tools.registry import tool

PROJECT_ROOT=Path.cwd().resolve()

PROTECTED_FILES = {
    ".env",
    ".git",
    ".venv",
    ".db",
}

IGNORE_DIRS = {
    ".git",
    ".venv",
    "__pycache__",
    "target",
    "node_modules",
    "data",
    ".env",
}


MAX_FILE_SIZE = 200_000

def resolve_project_path(path:str)->Path:
    target=(PROJECT_ROOT/path).resolve()

    try:
        target.relative_to(PROJECT_ROOT)
    except ValueError:
        raise HTTPException(status_code=400, detail="path is outside project")

    return target

def ensure_protected_file(target:str|Path)->None:
    target = Path(target).resolve()
    relative_path = target.relative_to(PROJECT_ROOT)

    if any(part in PROTECTED_FILES for part in relative_path.parts):
        raise HTTPException(
            status_code=403,
            detail="cant query the protected file"
        )


@tool("list files inside the current project")
def list_files()->list[str]:
    files=[]

    for path in PROJECT_ROOT.rglob("*"):
        if any(part in IGNORE_DIRS for part in path.parts):
            continue

        if path.is_file():
            files.append(path.relative_to(PROJECT_ROOT).as_posix())
    return files

@tool("read a utf-8 text file form the project")
def read_file(path:str)->dict[str,object]:
    target=resolve_project_path(path)

    ensure_protected_file(target)

    if not target.exists():
        raise HTTPException(status_code=404,detail="file not found")
    
    if not target.is_file():
        raise HTTPException(status_code=400,detail="path is not a file")
    
    size=target.stat().st_size
    if size>MAX_FILE_SIZE:
        raise HTTPException(status_code=400,detail="file is too large")
    
    try:
        content=target.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        raise HTTPException(status_code=400,detail="file is not utf-8 text")
    
    return {
        "path":target.relative_to(PROJECT_ROOT).as_posix(),
        "size":size,
        "content":content,
    }

@tool("search for text inside project file")
def search_text(query:str)-> list[dict[str,object]]:

    results=[]

    for path in PROJECT_ROOT.rglob("*"):
        if any(part in IGNORE_DIRS for part in path.parts ):
            continue
        
        if not path.is_file():
            continue

        if path.stat().st_size>MAX_FILE_SIZE:
            continue

        try:
            lines=path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue

        for line_no,line in enumerate(lines,start=1):
            if query in line:
                results.append(
                   {
                    "path": path.relative_to(PROJECT_ROOT).as_posix(),
                    "line": line_no,
                    "text": line.strip(),
                    }
                )
    return results

