"""Message context construction and history compaction."""
from typing import Any

from app.agent.prompts import SYSTEM_PROMPT
from app.history import get_messages


MAX_HISTORY_MESSAGES = 30

def build_agent_context()->list[dict[str,Any]]:
    history=get_messages()

    recent_history=history[-MAX_HISTORY_MESSAGES:]

    return [
        {
            "role":"system",
            "content":SYSTEM_PROMPT,

        },
        *recent_history
    ]


