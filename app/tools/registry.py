from typing import Any, Callable

from fastapi import HTTPException

from app.tools.read_files import list_files, read_file, search_text
from app.tools.write_files import edit_file, write_file


ToolFunction = Callable[..., Any]


TOOL_FUNCTIONS: dict[str, ToolFunction] = {
    "list_files": list_files,
    "read_file": read_file,
    "search_text": search_text,
    "write_file": write_file,
    "edit_file": edit_file,
}


TOOL_SCHEMAS = [
    {
        "type": "function",
        "function": {
            "name": "list_files",
            "description": "List files inside the current project.",
            "parameters": {
                "type": "object",
                "properties": {},
                "additionalProperties": False,
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a UTF-8 text file from the project.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Project-relative file path.",
                    }
                },
                "required": ["path"],
                "additionalProperties": False,
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "search_text",
            "description": "Search for text inside project files.",
            "parameters": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for.",
                    }
                },
                "required": ["query"],
                "additionalProperties": False,
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "write_file",
            "description": "Create or completely replace a project file.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                },
                "required": ["path", "content"],
                "additionalProperties": False,
            },
        },
    },
    {
        "type": "function",
        "function": {
            "name": "edit_file",
            "description": "Replace the first occurrence of text in a file.",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "old": {"type": "string"},
                    "new": {"type": "string"},
                },
                "required": ["path", "old", "new"],
                "additionalProperties": False,
            },
        },
    },
]


def dispatch_tool(name: str, arguments: dict[str, Any]) -> Any:
    tool = TOOL_FUNCTIONS.get(name)

    if tool is None:
        return {
            "ok": False,
            "error": f"Unknown tool: {name}",
        }

    try:
        result = tool(**arguments)

        return {
            "ok": True,
            "result": result,
        }

    except HTTPException as exc:
        return {
            "ok": False,
            "error": exc.detail,
        }

    except TypeError as exc:
        return {
            "ok": False,
            "error": f"Invalid tool arguments: {exc}",
        }

    except Exception as exc:
        return {
            "ok": False,
            "error": str(exc),
        }