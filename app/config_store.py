import os

from dotenv import load_dotenv

from app.schemas import LLMConfig


def save_llm_config(config: LLMConfig) -> None:
    """Persist the LLM configuration to the local .env file."""

    with open(".env", "w", encoding="utf-8") as f:
        f.write(f"model={config.model}\n")
        f.write(f"base_url={config.base_url}\n")
        f.write(f"api_key={config.api_key}\n")


def get_config_status() -> dict[str, object]:
    """Return public config status without exposing the API key value."""

    load_dotenv(override=True)

    return {
        "api_key_exist": bool(os.getenv("api_key")),
        "model": os.getenv("model"),
        "base_url": os.getenv("base_url"),
    }


def load_llm_config() -> tuple[str | None, str | None, str | None]:
    """Load raw LLM config values used to create the API client."""

    load_dotenv(override=True)

    api_key = os.getenv("api_key")
    base_url = os.getenv("base_url")
    model = os.getenv("model")

    return api_key, base_url, model
