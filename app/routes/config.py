from fastapi import APIRouter

from app.config_store import get_config_status, save_llm_config
from app.schemas import LLMConfig


router = APIRouter()


@router.post("/config")
async def save_config(config: LLMConfig):
    """Save model, base URL, and API key configuration."""

    save_llm_config(config)
    return {"message": "config has been saved"}


@router.get("/config/status")
def config_status():
    """Return the current public configuration status."""

    return get_config_status()
