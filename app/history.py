"""In-memory chat history.

The history is reset when the FastAPI process restarts. A later version can
replace this module with JSON, SQLite, or another persistence layer.
"""

chat_history: list[dict[str, str]] = []


def get_messages() -> list[dict[str, str]]:
    """Return the current conversation history."""

    return chat_history


def add_message(role: str, content: str) -> None:
    """Append one message to the conversation history."""

    chat_history.append({
        "role": role,
        "content": content,
    })


def pop_last_user_message() -> None:
    """Remove the last user message after a failed LLM request."""

    if chat_history and chat_history[-1]["role"] == "user":
        chat_history.pop()


def clear_messages() -> None:
    """Clear the current conversation history."""

    chat_history.clear()
