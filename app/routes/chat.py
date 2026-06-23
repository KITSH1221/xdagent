from fastapi import APIRouter, HTTPException
from fastapi.responses import StreamingResponse

from app.agent.loop import run_agent
from app.history import clear_messages, get_messages
# from app.llm import chat_once, chat_stream
from app.schemas import ChatRequest


router = APIRouter()


@router.get("/history")
def get_history():
    """Return the current in-memory chat history."""

    return {"messages": get_messages()}


@router.delete("/history")
def clear_history():
    """Clear the current in-memory chat history."""

    clear_messages()
    return {"message": "history cleared"}


@router.post("/chat")
def chatbot(request: ChatRequest):
    """Handle one non-streaming chat request."""

    user_message = request.message.strip()
    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")


    return {"message":run_agent(user_message)}


@router.post("/chat/stream")
def chatbot_stream(request: ChatRequest):
    """Handle one streaming chat request."""

    user_message = request.message.strip()
    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")

    def generator():
        yield run_agent(user_message)

    return StreamingResponse(generator(), media_type="text/plain")
