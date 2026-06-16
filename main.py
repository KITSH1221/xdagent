import os
from fastapi import FastAPI
from dotenv import load_dotenv
from openai import OpenAI
from pydantic import BaseModel

app = FastAPI()


class LLMConfig(BaseModel):
    model: str
    base_url: str
    api_key: str


class ChatRequest(BaseModel):
    message: str


@app.post("/config")
async def save_config(config: LLMConfig):
    with open(".env", "w", encoding="utf-8") as f:
        f.write(f"model={config.model}\n")
        f.write(f"base_url={config.base_url}\n")
        f.write(f"api_key={config.api_key}\n")

    return {"message": "config has been saved"}


@app.post("/chat")
def chatbot(request: ChatRequest):
    load_dotenv(override=True)

    client = OpenAI(
        api_key=os.getenv("api_key"),
        base_url=os.getenv("base_url"),
    )

    response = client.chat.completions.create(
        model=os.getenv("model"),
        messages=[
            {"role": "system", "content": "You are a helpful assistant"},
            {"role": "user", "content": request.message},
        ],
        stream=False,
        timeout=120,
    )

    return {"message": response.choices[0].message.content}