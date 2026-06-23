"""Agent loop orchestration will be implemented in this module."""
import json
from typing import Any

from fastapi import HTTPException
from openai import OpenAIError

from app.agent.context import build_agent_context
from app.history import add_message, pop_last_user_message
from app.llm import get_client_and_model
from app.tools.registry import dispatch_tool, get_tool_schemas


MAX_AGENT_STEPS=8
MAX_RESULT_LENGTH=50_000

def serialize_tool_result(result:Any)->str:
    content=json.dumps(
        result,
        ensure_ascii=False,
        default=str,
    )

    if len(content)>MAX_RESULT_LENGTH:
        return content[:MAX_RESULT_LENGTH]+"\n result tool truncated"
    
    return content


def run_agent(user_message:str)->str:
    client,model=get_client_and_model()

    tools=get_tool_schemas()
    if not tools:
        raise HTTPException(
            status_code=500,
            detail="the tool are not registed"
        )
    
    print(
    "registered tools:",
    [item["function"]["name"] for item in tools],
)
    add_message("user",user_message)
    messages=build_agent_context()

    try:
        for _ in range(MAX_AGENT_STEPS):
            response=client.chat.completions.create(
                model=model,
                messages=messages,
                tools=tools,
                tool_choice="auto",
                stream=False,
                timeout=120,
            )

            message=response.choices[0].message

            if not message.tool_calls :
                final_answer=message.content or ""
                add_message("assistant",final_answer)

                return final_answer

            assistant_tool_message={
                "role":"assistant",
                "content":message.content,
                "tool_calls":
                [
                    {
                        "id": tool_call.id,
                        "type": "function",
                        "function": {
                            "name": tool_call.function.name,
                            "arguments": tool_call.function.arguments,
                        },
                    }
                    for tool_call in message.tool_calls
                ],
            } 

            messages.append(assistant_tool_message)

            for tool_call in message.tool_calls:
                tool_name = tool_call.function.name

                try:
                    arguments = json.loads(
                        tool_call.function.arguments or "{}"
                    )
                except json.JSONDecodeError as exc:
                    tool_result = {
                        "ok": False,
                        "error": f"Invalid tool JSON: {exc}",
                    }
                else:
                    tool_result = dispatch_tool(
                        tool_name,
                        arguments,
                    )

                messages.append(
                    {
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "name": tool_name,
                        "content": serialize_tool_result(tool_result),
                    }
                )

        final_answer = (
            "Agent stopped because it reached the maximum "
            f"number of steps ({MAX_AGENT_STEPS})."
        )
        add_message("assistant", final_answer)
        return final_answer

    except OpenAIError as exc:
        pop_last_user_message()

        raise HTTPException(
            status_code=502,
            detail=f"LLM request failed: {exc}",
        ) from exc

    except Exception as exc:
        pop_last_user_message()

        raise HTTPException(
            status_code=500,
            detail=f"Agent failed: {exc}",
        ) from exc
