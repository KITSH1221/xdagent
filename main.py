import os
from typing import Literal

from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException
from fastapi.responses import StreamingResponse
from openai import OpenAI, OpenAIError
from pydantic import BaseModel


# 创建 FastAPI 应用
app = FastAPI()

# 系统提示词：决定助手的基础身份
SYSTEM_PROMPT = "You are XD Agent, a helpful coding assistant."

# 内存中的聊天历史
# 服务重启后会丢失，后面可以升级成保存到 json 或数据库
chat_history: list[dict[str, str]] = []


# 模型配置请求体
class LLMConfig(BaseModel):
    model: str
    base_url: str
    api_key: str


# 聊天请求体
class ChatRequest(BaseModel):
    message: str


# 单条消息结构
class Message(BaseModel):
    role: Literal["system", "user", "assistant"]
    content: str


# 保存模型配置到 .env
@app.post("/config")
async def save_config(config: LLMConfig):
    with open(".env", "w", encoding="utf-8") as f:
        f.write(f"model={config.model}\n")
        f.write(f"base_url={config.base_url}\n")
        f.write(f"api_key={config.api_key}\n")

    return {"message": "config has been saved"}


# 查看配置状态
@app.get("/config/status")
def config_status():
    load_dotenv(override=True)

    return {
        # 不返回真实 api_key，只返回是否存在
        "api_key_exist": bool(os.getenv("api_key")),
        "model": os.getenv("model"),
        "base_url": os.getenv("base_url"),
    }


# 查看当前聊天历史
@app.get("/history")
def get_history():
    return {"messages": chat_history}


# 清空当前聊天历史
@app.delete("/history")
def clear_history():
    chat_history.clear()
    return {"message": "history cleared"}


# 读取 .env，并创建 OpenAI 客户端
def get_client_and_model():
    load_dotenv(override=True)

    api_key = os.getenv("api_key")
    base_url = os.getenv("base_url")
    model = os.getenv("model")

    if not api_key or not base_url or not model:
        raise HTTPException(status_code=400, detail="cant find the model")

    client = OpenAI(
        api_key=api_key,
        base_url=base_url,
    )

    return client, model


# 普通聊天接口：一次性返回完整回答
@app.post("/chat")
def chatbot(request: ChatRequest):
    client, model = get_client_and_model()

    user_message = request.message.strip()
    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")

    # 加入用户消息
    chat_history.append({
        "role": "user",
        "content": user_message,
    })

    # system prompt + 历史消息
    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        *chat_history,
    ]

    try:
        response = client.chat.completions.create(
            model=model,
            messages=messages,
            stream=False,
            timeout=120,
        )

        assistant_message = response.choices[0].message.content or ""

        # 加入助手回答
        chat_history.append({
            "role": "assistant",
            "content": assistant_message,
        })

        return {"message": assistant_message}

    except OpenAIError as e:
        # 请求失败时移除刚刚加入的用户消息
        if chat_history and chat_history[-1]["role"] == "user":
            chat_history.pop()

        raise HTTPException(status_code=500, detail=str(e))


# 流式聊天接口：边生成边返回
@app.post("/chat/stream")
def chatbot_stream(request: ChatRequest):
    client, model = get_client_and_model()

    user_message = request.message.strip()
    if not user_message:
        raise HTTPException(status_code=400, detail="message is empty")

    chat_history.append({
        "role": "user",
        "content": user_message,
    })

    messages = [
        {"role": "system", "content": SYSTEM_PROMPT},
        *chat_history,
    ]

    def generate():
        full_message = ""

        try:
            response = client.chat.completions.create(
                model=model,
                messages=messages,
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

            chat_history.append({
                "role": "assistant",
                "content": full_message,
            })

        except OpenAIError as e:
            if chat_history and chat_history[-1]["role"] == "user":
                chat_history.pop()

            yield f"\n[error] {str(e)}"

        except Exception as e:
            if chat_history and chat_history[-1]["role"] == "user":
                chat_history.pop()

            yield f"\n[error] {str(e)}"

    return StreamingResponse(generate(), media_type="text/plain")