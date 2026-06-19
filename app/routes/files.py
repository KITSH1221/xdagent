from fastapi import APIRouter

from app.tools.read_files import list_files, read_file


router = APIRouter(prefix="/files", tags=["files"])


@router.get("/tree")
def files_tree():
    return {
        "files": list_files(),
    }


@router.get("/read")
def files_read(path: str):
    return read_file(path)