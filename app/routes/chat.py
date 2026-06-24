from fastapi import APIRouter, HTTPException
from fastapi.responses import StreamingResponse

from app.agent.loop import run_agent
from app.history import clear_messages, get_messages,DEFAULT_CONVERSATION_ID,get_conversation
# from app.llm import chat_once, chat_stream
from app.schemas import ChatRequest


router = APIRouter()
def ensure_conversation_exists(conversation_id:str)->None:
    try:
        get_conversation(conversation_id)
    except ValueError as e:
        raise HTTPException(
            # FIX: an unknown conversation is a missing resource.
            status_code=404,
            detail=str(e)
        ) from e

@router.get("/history")
def get_history(conversation_id:str=DEFAULT_CONVERSATION_ID):
    """Return the current in-memory chat history."""

    return {"messages": get_messages(conversation_id)}


@router.delete("/history")
def clear_history(conversation_id:str=DEFAULT_CONVERSATION_ID):
    """Clear the current in-memory chat history."""

    clear_messages(conversation_id)
    return {"message": "history cleared",
            "conversation_id":conversation_id,
            }


@router.post("/chat")
def chatbot(request: ChatRequest):
    """Handle one non-streaming chat request."""

    user_message = request.message.strip()

    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")

    ensure_conversation_exists(request.conversation_id)

    return {"message":run_agent(user_message,request.conversation_id)}


@router.post("/chat/stream")
def chatbot_stream(request: ChatRequest):
    """Handle one streaming chat request."""

    user_message = request.message.strip()
    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")
    # FIX: validate before StreamingResponse begins sending a 200 response.
    ensure_conversation_exists(request.conversation_id)
    def generator():
        yield run_agent(user_message,request.conversation_id)

    return StreamingResponse(generator(), media_type="text/plain")
