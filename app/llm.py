from collections.abc import Iterator

from fastapi import HTTPException
from openai import OpenAI, OpenAIError

from app.config_store import load_llm_config
from app.history import add_message, get_messages, pop_last_user_message

from app.tools.read_files import read_file,list_files,search_text



SYSTEM_PROMPT = "You are XD Agent, a helpful coding assistant."


def get_client_and_model() -> tuple[OpenAI, str]:
    """Create an OpenAI-compatible client from the saved config."""

    api_key, base_url, model = load_llm_config()

    if not api_key or not base_url or not model:
        raise HTTPException(status_code=400, detail="cant find the model")

    client = OpenAI(
        api_key=api_key,
        base_url=base_url,
    )

    return client, model


def build_messages() -> list[dict[str, str]]:
    """Build the message list sent to the LLM API."""

    return [
        {"role": "system", "content": SYSTEM_PROMPT},
        *get_messages(),
    ]


def chat_once(user_message: str) -> str:
    """Send one non-streaming chat request and store the assistant reply."""
    local_result = handle_local_command(user_message)
    if local_result is not None:
        return local_result
    client, model = get_client_and_model()
    
    add_message("user", user_message)

    try:
        response = client.chat.completions.create(
            model=model,
            messages=build_messages(),
            stream=False,
            timeout=120,
        )

        assistant_message = response.choices[0].message.content or ""
        add_message("assistant", assistant_message)

        return assistant_message

    except OpenAIError as e:
        pop_last_user_message()
        raise HTTPException(status_code=500, detail=str(e)) from e


def chat_stream(user_message: str) -> Iterator[str]:
    """Stream an assistant reply while collecting it into chat history."""
    local_result = handle_local_command(user_message)
    if local_result is not None:
        yield local_result
        return 
    
    client, model = get_client_and_model()
    add_message("user", user_message)
    full_message = ""

    try:
        response = client.chat.completions.create(
            model=model,
            messages=build_messages(),
            stream=True,
            timeout=120,
        )

        for chunk in response:
            if not chunk.choices:
                continue

            delta = chunk.choices[0].delta.content

            if delta:
                full_message += delta
                yield delta

        add_message("assistant", full_message)

    except OpenAIError as e:
        pop_last_user_message()
        yield f"\n[error] {str(e)}"

    except Exception as e:
        pop_last_user_message()
        yield f"\n[error] {str(e)}"


def handle_local_command(user_message: str) -> str | None:
    user_message = user_message.strip()

    if user_message.startswith("/search "):
        query = user_message.removeprefix("/search ").strip()
        if not query:
            return "Usage: /search keyword"

        results = search_text(query)

        if not results:
            return f"No matches for: {query}"

        lines = [
            f"{item['path']}:{item['line']}: {item['text']}"
            for item in results[:100]
        ]

        if len(results) > 100:
            lines.append(f"... {len(results) - 100} more matches")

        return "\n".join(lines)

    if user_message == "/tree":
        files = list_files()

        if not files:
            return "No files found."

        return "Project files:\n" + "\n".join(files)

    if user_message.startswith("/tree "):
        return "Usage: /tree"

    if user_message.startswith("/read "):
        path = user_message.removeprefix("/read ").strip()
        if not path:
            return "Usage: /read path/to/file"

        file_data = read_file(path)

        return (
            f"File: {file_data['path']}\n"
            f"Size: {file_data['size']} bytes\n\n"
            f"{file_data['content']}"
        )

    return None