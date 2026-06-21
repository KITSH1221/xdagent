from fastapi import APIRouter

from app.tools.read_files import list_files, read_file
from app.tools.write_files import edit_file,write_file


router = APIRouter(prefix="/files", tags=["files"])


@router.get("/tree")
def files_tree():
    return {
        "files": list_files(),
    }


@router.get("/read")
def files_read(path: str):
    return read_file(path)

@router.post("/write")
def files_write(path:str,content:str):
    return write_file(path,content)

@router.post("/write")
def files_edit(path:str,old:str,new:str):
    return edit_file(path,old,new)