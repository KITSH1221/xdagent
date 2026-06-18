from fastapi import FastAPI

from app.routes import chat, config


app = FastAPI()

app.include_router(config.router)
app.include_router(chat.router)
