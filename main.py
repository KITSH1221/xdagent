from contextlib import asynccontextmanager

from fastapi import FastAPI

from app.history import init_db
from app.routes import chat, config, files


@asynccontextmanager
async def lifespan(app: FastAPI):
    print("服务启动")
    init_db()
    yield
    print("服务关闭")


app = FastAPI(lifespan=lifespan)

app.include_router(files.router)
app.include_router(config.router)
app.include_router(chat.router)
