from fastapi import HTTPException
from openai import OpenAI

from app.config_store import load_llm_config


def get_client_and_model() -> tuple[OpenAI, str]:
    api_key, base_url, model = load_llm_config()

    if not api_key or not base_url or not model:
        raise HTTPException(
            status_code=400,
            detail="Cannot find the model configuration.",
        )

    client = OpenAI(
        api_key=api_key,
        base_url=base_url,
    )

    return client, model
