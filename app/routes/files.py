from fastapi import APIRouter

from app.schemas import EditFileRequest, WriteFileRequest
from app.tools.read_files import list_files, read_file
from app.tools.write_files import edit_file, write_file

router = APIRouter(prefix="/files", tags=["files"])


@router.get("/tree")
def files_tree():
    return {"files": list_files()}


@router.get("/read")
def files_read(path: str):
    return read_file(path)


@router.post("/write")
def files_write(request: WriteFileRequest):
    return write_file(request.path, request.content)


@router.post("/edit")
def files_edit(request: EditFileRequest):
    return edit_file(request.path, request.old, request.new)
