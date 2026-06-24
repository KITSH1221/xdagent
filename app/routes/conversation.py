from fastapi import APIRouter, HTTPException

from app.history import (
    create_conversation,
    get_conversation,
    get_message_path,
    get_message_tree,
    list_conversations,
    switch_active_leaf,
)
from app.schemas import (
    CreateConversationRequest,
    SwitchLeafRequest,
)


router = APIRouter(
    prefix="/conversations",
    tags=["conversations"],
)


@router.post("", status_code=201)
def conversation_create(request: CreateConversationRequest):
    try:
        return create_conversation(
            title=request.title,
            workspace_path=request.workspace_path,
        )
    except ValueError as exc:
        raise HTTPException(
            status_code=400,
            detail=str(exc),
        ) from exc


@router.get("")
def conversation_list():
    return {
        "conversations": list_conversations(),
    }


@router.get("/{conversation_id}")
def conversation_detail(conversation_id: str):
    try:
        return get_conversation(conversation_id)
    except ValueError as exc:
        raise HTTPException(
            status_code=404,
            detail=str(exc),
        ) from exc


@router.get("/{conversation_id}/tree")
def conversation_tree(conversation_id: str):
    try:
        conversation = get_conversation(conversation_id)
        messages = get_message_tree(conversation_id)
    except ValueError as exc:
        raise HTTPException(
            status_code=404,
            detail=str(exc),
        ) from exc

    return {
        "conversation": conversation,
        "messages": messages,
    }


@router.get("/{conversation_id}/path")
def conversation_path(conversation_id: str):
    try:
        messages = get_message_path(conversation_id)
    except ValueError as exc:
        raise HTTPException(
            status_code=404,
            detail=str(exc),
        ) from exc

    return {
        "conversation_id": conversation_id,
        "leaf_id": messages[-1]["id"] if messages else None,
        "messages": messages,
    }


@router.patch("/{conversation_id}/active-leaf")
def conversation_switch_leaf(
    conversation_id: str,
    request: SwitchLeafRequest,
):
    try:
        switch_active_leaf(
            message_id=request.message_id,
            conversation_id=conversation_id,
        )

        messages = get_message_path(conversation_id)
    except ValueError as exc:
        raise HTTPException(
            status_code=404,
            detail=str(exc),
        ) from exc

    return {
        "conversation_id": conversation_id,
        "active_leaf_id": request.message_id,
        "messages": messages,
    }