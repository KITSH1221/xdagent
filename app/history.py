import json
import sqlite3
from pathlib import Path
from typing import Any
from uuid import uuid4


PROJECT_ROOT = Path.cwd().resolve()
DB_PATH = PROJECT_ROOT / "data" / "xdagent.db"

def generate_id()->str:
    return uuid4().hex


def get_conn() -> sqlite3.Connection:
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    
    conn.execute("PRAGMA foreign_keys = ON")

    return conn


def init_db() -> None:
    with get_conn() as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                active_leaf_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            """
        )

        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                parent_id TEXT,
                role TEXT NOT NULL,
                content TEXT,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                FOREIGN KEY (conversation_id)
                    REFERENCES conversations(id)
                    ON DELETE CASCADE,

                FOREIGN KEY (parent_id)
                    REFERENCES messages(id)
                    ON DELETE RESTRICT
            )
            """
        )

        conn.execute(
            """
            CREATE INDEX IF NOT EXISTS idx_messages_conversation
            ON messages(conversation_id)
            """
        )

        conn.execute(
            """
            CREATE INDEX IF NOT EXISTS idx_messages_parent
            ON messages(parent_id)
            """
        )

def get_messages() -> list[dict[str, str]]:
    """Return the current conversation history."""
    #init_db()
    with get_conn() as conn:
        rows = conn.execute(
            "SELECT role, content FROM messages ORDER BY id ASC"
        ).fetchall()
    return [
        {"role": row["role"], "content": row["content"]}
        for row in rows
    ]


def add_message(role: str, content: str) -> None:
    """Append one message to the conversation history."""
    #init_db()
    with get_conn() as conn:
        conn.execute(
            "INSERT INTO messages (role, content) VALUES (?, ?)",
            (role, content),
        )


def pop_last_user_message() -> None:
    """Remove the last message only if it is a user message."""
    #init_db()
    with get_conn() as conn:
        row = conn.execute(
            "SELECT id, role FROM messages ORDER BY id DESC LIMIT 1"
        ).fetchone()
        if row is not None and row["role"] == "user":
            conn.execute("DELETE FROM messages WHERE id = ?", (row["id"],))


def clear_messages() -> None:
    """Clear the current conversation history."""
    #init_db()
    with get_conn() as conn:
        conn.execute("DELETE FROM messages")
