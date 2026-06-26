"""Agent loop orchestration will be implemented in this module."""
import json
from contextlib import nullcontext
from typing import Any

from fastapi import HTTPException
from openai import OpenAIError

from app.llm import get_client_and_model
from app.agent.context import build_agent_context
from app.history import add_message, pop_last_user_message,get_conversation
from app.tools.registry import dispatch_tool, get_tool_schemas
from app.workspace import bind_workspace
from app.approvals import create_approval

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


def run_agent(user_message:str,conversation_id:str="default")->str:
    client,model=get_client_and_model()
    conversation=get_conversation(conversation_id)

    # FIX: only workspace conversations receive file tools and a bound root.
    if conversation["mode"] == "workspace":
        workspace_path = conversation["workspace_path"]
        if not isinstance(workspace_path, str) or not workspace_path:
            raise HTTPException(
                status_code=500,
                detail="workspace conversation has no workspace path",
            )

        tools=get_tool_schemas()
        if not tools:
            raise HTTPException(
                status_code=500,
                detail="the tools are not registered",
            )
        workspace_context=bind_workspace(workspace_path)
    else:
        # FIX: general conversations run without a filesystem workspace.
        tools=[]
        workspace_context=nullcontext()

    add_message("user",user_message,conversation_id)
    messages=build_agent_context(conversation_id)

    try:
        with workspace_context:
            for _ in range(MAX_AGENT_STEPS):
                request_options = {
                    "model": model,
                    "messages": messages,
                    "stream": False,
                    "timeout": 120,
                }

                # FIX: omit tool parameters entirely for general conversations.
                if tools:
                    request_options["tools"] = tools
                    request_options["tool_choice"] = "auto"

                response=client.chat.completions.create(**request_options)

                message=response.choices[0].message

                if not message.tool_calls :
                    final_answer=message.content or ""
                    add_message("assistant",final_answer,conversation_id)

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
                        if tool_result.get("requires_approval"):
                            approval = create_approval(
                                conversation_id=conversation_id,
                                workspace_path=workspace_path,
                                tool=tool_name,
                                arguments=arguments,
                            )

                            final_answer = (
                                "This operation requires approval.\n\n"
                                f"approval_id: {approval.id}\n"
                                f"tool: {tool_name}\n"
                                f"arguments: {json.dumps(arguments, ensure_ascii=False)}\n\n"
                                "Type /approve to execute it, or /deny to cancel."
                            )

                            add_message("assistant", final_answer, conversation_id)
                            return final_answer

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
            add_message("assistant", final_answer,conversation_id)
            return final_answer

    except OpenAIError as exc:
        pop_last_user_message(conversation_id)

        raise HTTPException(
            status_code=502,
            detail=f"LLM request failed: {exc}",
        ) from exc

    except Exception as exc:
        pop_last_user_message(conversation_id)

        raise HTTPException(
            status_code=500,
            detail=f"Agent failed: {exc}",
        ) from exc
