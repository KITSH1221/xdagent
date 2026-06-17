import os
from fastapi import FastAPI
from dotenv import load_dotenv
from openai import OpenAI,OpenAIError
from pydantic import BaseModel
from fastapi import HTTPException


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

@app.get("/config/status")
def config_status():
    load_dotenv(override=True)

    return {
        "api_key":bool(os.getenv("api_key")),
        "model":os.getenv("base_model"),
        "base_url":os.getenv("base_url"),
    }
@app.post("/chat")
def chatbot(request: ChatRequest):
    load_dotenv(override=True)
    api_key=os.getenv("api_key")
    base_url=os.getenv("base_url")
    model=os.getenv("model")

    if not api_key or not base_url or not model :
        raise HTTPException(status_code=400,detail="cant find the model")
    try:
        client = OpenAI(
            api_key=api_key,
            base_url=base_url,
        )

        response = client.chat.completions.create(
            model=model,
            messages=[
                {"role": "system", "content": "You are a helpful assistant"},
                {"role": "user", "content": request.message},
            ],
            stream=False,
            timeout=120,
        )

        return {"message": response.choices[0].message.content}
    except OpenAIError as e:
        raise e
