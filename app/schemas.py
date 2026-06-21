from typing import Literal

from pydantic import BaseModel


class LLMConfig(BaseModel):
    """Request body for saving the model provider configuration."""

    model: str
    base_url: str
    api_key: str


class ChatRequest(BaseModel):
    """Request body for a user chat message."""

    message: str


class Message(BaseModel):
    """Internal chat message shape sent to the LLM API."""

    role: Literal["system", "user", "assistant"]
    content: str

class WriteFileRequest(BaseModel):
    path:str
    content:str

class EditFileRequest(BaseModel):
    path:str
    old:str
    new:str
    