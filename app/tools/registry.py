import inspect
from typing import Any, Callable,get_type_hints


MUTATING_TOOLS=[
    "write_file",
    "edit_file"
]

ToolFunction = Callable[..., Any]

_TOOL_FUNCTIONS:dict[str,ToolFunction]={}
_TOOL_DESCRIPTIONS:dict[str,str]={}

def tool(description:str):

    def wrapper(func:ToolFunction)->ToolFunction:
        _TOOL_FUNCTIONS[func.__name__]=func
        _TOOL_DESCRIPTIONS[func.__name__]=description
        return func
    
    return wrapper

def python_type_json_type(annotation:Any)->str:
    if annotation is str:
        return "string"
    if annotation is int:
        return "integer"
    if annotation is float:
        return "number"
    if annotation is bool:
        return "boolean"
    
    return "string"

def build_tool_schema(name:str,func:ToolFunction)->dict[str,Any]:
    signature=inspect.signature(func)
    hints=get_type_hints(func)

    properties={}
    required=[]


    for param_name,param in signature.parameters.items():

        annotation=hints.get(param_name,str)

        properties[param_name]={
            "type":python_type_json_type(annotation),
        }
        if param.default is inspect.Parameter.empty:
            required.append(param_name)

    return {
        "type": "function",
        "function": {
            "name": name,
            "description": _TOOL_DESCRIPTIONS.get(name, ""),
            "parameters": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": False,
            },
        },
    }



def get_tool_schemas()->list[dict[str,Any]]:
    return [
        build_tool_schema(name,func)
        for name,func in _TOOL_FUNCTIONS.items()
    ]


def dispatch_tool(name: str, arguments: dict[str, Any],allow_write:bool=False) -> Any:
    if name in MUTATING_TOOLS and not allow_write:
        return {
            "ok": False,
            "requires_approval": True,
            "tool": name,
            "arguments": arguments,
            "error": "User approval is required",
        }
    
    
    tool_func=_TOOL_FUNCTIONS.get(name)

    if tool_func is None:
        return {
            "ok": False,
            "error": f"Unknown tool: {name}",
        }

    try:
        result = tool_func(**arguments)

        return {
            "ok": True,
            "result": result,
        }

    except Exception as exc:
        return {
            "ok": False,
            "error": str(exc),
        }