from fastapi import APIRouter, HTTPException

from app.approvals import pop_approval
from app.history import add_message
from app.tools.registry import dispatch_tool
from app.workspace import bind_workspace


router = APIRouter(
    prefix="/approvals",
    tags=["approvals"],
)


@router.post("/{approval_id}/approve")
def approve_tool(approval_id: str):
    try:
        approval = pop_approval(approval_id)
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc

    with bind_workspace(approval.workspace_path):
        result = dispatch_tool(
            approval.tool,
            approval.arguments,
            allow_write=True,
        )

    add_message(
        "assistant",
        f"Approved and executed `{approval.tool}`.\n\nResult:\n{result}",
        approval.conversation_id,
    )

    return {
        "approval_id": approval_id,
        "tool": approval.tool,
        "result": result,
    }


@router.post("/{approval_id}/deny")
def deny_tool(approval_id: str):
    try:
        approval = pop_approval(approval_id)
    except ValueError as exc:
        raise HTTPException(status_code=404, detail=str(exc)) from exc

    add_message(
        "assistant",
        f"Denied `{approval.tool}`.",
        approval.conversation_id,
    )

    return {
        "approval_id": approval_id,
        "tool": approval.tool,
        "denied": True,
    }