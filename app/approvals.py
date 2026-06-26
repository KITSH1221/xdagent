from dataclasses import dataclass
from uuid import uuid4
from typing import Any

@dataclass
class PendingApproval:
    id: str
    conversation_id: str
    workspace_path: str
    tool: str
    arguments: dict[str, Any]


_PENDING_APPROVALS: dict[str, PendingApproval] = {}


def create_approval(
    *,
    conversation_id: str,
    workspace_path: str,
    tool: str,
    arguments: dict[str, Any],
) -> PendingApproval:
    approval = PendingApproval(
        id=uuid4().hex,
        conversation_id=conversation_id,
        workspace_path=workspace_path,
        tool=tool,
        arguments=arguments,
    )
    _PENDING_APPROVALS[approval.id] = approval
    return approval


def get_approval(approval_id:str)->PendingApproval:
    approval=_PENDING_APPROVALS.get(approval_id)

    if approval is None:
        raise ValueError(f"the approval_id doesnt exist {approval_id}")
    return approval


def pop_approval(approval_id: str) -> PendingApproval:
    approval = _PENDING_APPROVALS.pop(approval_id, None)
    if approval is None:
        raise ValueError(f"approval does not exist: {approval_id}")
    return approval
