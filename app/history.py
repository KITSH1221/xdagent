import sqlite3
from pathlib import Path
from uuid import uuid4


PROJECT_ROOT = Path(__file__).resolve().parents[1]
DB_PATH = PROJECT_ROOT / "data" / "xdagent.db"

DEFAULT_CONVERSATION_ID="default"

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
                mode TEXT NOT NULL,
                workspace_path TEXT,
                active_leaf_id TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,

                CHECK (mode IN ('general', 'workspace'))
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
                    ON DELETE CASCADE
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
        ensure_default_conversation(conn)




def ensure_default_conversation(conn: sqlite3.Connection) -> None:
    row = conn.execute(
        """
        SELECT id
        FROM conversations
        WHERE id = ?
        """,
        (DEFAULT_CONVERSATION_ID,),
    ).fetchone()

    if row is not None:
        return

    conn.execute(
        """
        INSERT INTO conversations (
            id,
            title,
            mode,
            workspace_path
        )
        VALUES (?, ?, ?, ?)
        """,
        (
            DEFAULT_CONVERSATION_ID,
            "Default",
            "workspace",
            str(PROJECT_ROOT),
        ),
    )

def create_conversation(
    title: str,
    workspace_path: str | None = None,
) -> dict[str, object]:
    title = title.strip()

    if not title:
        raise ValueError("conversation title cannot be empty")

    normalized_workspace: str | None = None
    mode = "general"

    if workspace_path:
        workspace = Path(workspace_path).expanduser().resolve()

        if not workspace.exists():
            raise ValueError(
                f"workspace does not exist: {workspace}"
            )

        if not workspace.is_dir():
            raise ValueError(
                f"workspace is not a directory: {workspace}"
            )

        normalized_workspace = str(workspace)
        mode = "workspace"

    conversation_id = generate_id()

    with get_conn() as conn:
        conn.execute(
            """
            INSERT INTO conversations (
                id,
                title,
                mode,
                workspace_path
            )
            VALUES (?, ?, ?, ?)
            """,
            (
                conversation_id,
                title,
                mode,
                normalized_workspace,
            ),
        )

    return {
        "id": conversation_id,
        "title": title,
        "mode": mode,
        "workspace_path": normalized_workspace,
        "active_leaf_id": None,
    }


def get_conversation(
    conversation_id: str,
) -> dict[str, object]:
    with get_conn() as conn:
        row = conn.execute(
            """
            SELECT
                id,
                title,
                mode,
                workspace_path,
                active_leaf_id,
                created_at,
                updated_at
            FROM conversations
            WHERE id = ?
            """,
            (conversation_id,),
        ).fetchone()

    if row is None:
        raise ValueError(
            f"conversation does not exist: {conversation_id}"
        )

    return {
        "id": row["id"],
        "title": row["title"],
        "mode": row["mode"],
        "workspace_path": row["workspace_path"],
        "active_leaf_id": row["active_leaf_id"],
        "created_at": row["created_at"],
        "updated_at": row["updated_at"],
    }


def list_conversations() -> list[dict[str, object]]:
    with get_conn() as conn:
        rows = conn.execute(
            """
            SELECT
                id,
                title,
                mode,
                workspace_path,
                active_leaf_id,
                created_at,
                updated_at
            FROM conversations
            ORDER BY updated_at DESC, created_at DESC
            """
        ).fetchall()

    return [
        {
            "id": row["id"],
            "title": row["title"],
            "mode": row["mode"],
            "workspace_path": row["workspace_path"],
            "active_leaf_id": row["active_leaf_id"],
            "created_at": row["created_at"],
            "updated_at": row["updated_at"],
        }
        for row in rows
    ]


def ensure_conversation(
    conn: sqlite3.Connection,
    conversation_id: str,
) -> None:
    row = conn.execute(
        """
        SELECT id
        FROM conversations
        WHERE id = ?
        """,
        (conversation_id,),
    ).fetchone()

    if row is None:
        raise ValueError(
            f"conversation does not exist: {conversation_id}"
        )

def get_active_leaf_id(
    conn: sqlite3.Connection,
    conversation_id: str = DEFAULT_CONVERSATION_ID,
) -> str | None:
    row = conn.execute(
        """
        SELECT active_leaf_id
        FROM conversations
        WHERE id = ?
        """,
        (conversation_id,),
    ).fetchone()

    if row is None:
        raise ValueError("conversation does not exist")

    return row["active_leaf_id"]

def set_active_leaf_id(
    conn: sqlite3.Connection,
    message_id: str | None,
    conversation_id: str = DEFAULT_CONVERSATION_ID,
) -> None:
    cursor = conn.execute(
        """
        UPDATE conversations
        SET active_leaf_id = ?,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        """,
        (message_id, conversation_id),
    )

    if cursor.rowcount == 0:
        raise ValueError("conversation does not exist")

def add_message(role: str, content: str,conversation_id: str = DEFAULT_CONVERSATION_ID) -> str:
    """
    Add one message after the current active leaf.

    Returns the new message id.
    """
    message_id = generate_id()

    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        parent_id = get_active_leaf_id(conn, conversation_id)

        conn.execute(
            """
            INSERT INTO messages (
                id,
                conversation_id,
                parent_id,
                role,
                content
            )
            VALUES (?, ?, ?, ?, ?)
            """,
            (
                message_id,
                conversation_id,
                parent_id,
                role,
                content,
            ),
        )

        set_active_leaf_id(conn, message_id, conversation_id)

    return message_id



def get_messages(conversation_id: str = DEFAULT_CONVERSATION_ID,) -> list[dict[str, str]]:
    """
    Return the active branch history.

    This walks from active_leaf_id back through parent_id,
    then reverses the result into normal chat order.
    """
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        active_leaf_id = get_active_leaf_id(conn, conversation_id)

        if active_leaf_id is None:
            return []

        messages = []
        current_id = active_leaf_id

        while current_id is not None:
            row = conn.execute(
                """
                SELECT id, parent_id, role, content
                FROM messages
                WHERE id = ?
                  AND conversation_id = ?
                """,
                (current_id, conversation_id),
            ).fetchone()

            if row is None:
                break

            messages.append(
                {
                    "id": row["id"],
                    "role": row["role"],
                    "content": row["content"] or "",
                    "parent_id": row["parent_id"],
                }
            )

            current_id = row["parent_id"]

    messages.reverse()

    return [
        {
            "role": message["role"],
            "content": message["content"],
        }
        for message in messages
    ]
             

def get_message_path(conversation_id: str = DEFAULT_CONVERSATION_ID,)->list[dict[str,str | None]]:
    """
    Return the active branch with ids.

    Use this later for TUI branch rendering.
    """
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        active_leaf_id = get_active_leaf_id(conn, conversation_id)

        if active_leaf_id is None:
            return []

        messages = []
        current_id = active_leaf_id

        while current_id is not None:
            row = conn.execute(
                """
                SELECT id, parent_id, role, content, created_at
                FROM messages
                WHERE id = ?
                  AND conversation_id = ?
                """,
                (current_id, conversation_id),
            ).fetchone()

            if row is None:
                break

            messages.append(
                {
                    "id": row["id"],
                    "parent_id": row["parent_id"],
                    "role": row["role"],
                    "content": row["content"] or "",
                    "created_at": row["created_at"],
                }
            )

            current_id = row["parent_id"]

    messages.reverse()
    return messages


def get_message_tree(conversation_id: str = DEFAULT_CONVERSATION_ID,)->list[dict[str,str | None]]:
    """
    Return all messages in the conversation.

    This is a flat list with parent_id.
    The frontend/TUI can turn it into a tree.
    """
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        rows=conn.execute(
            """
            SELECT id, parent_id, role, content, created_at
            FROM messages
            WHERE conversation_id = ?
            ORDER BY created_at ASC
            """,
            (conversation_id,),
        ).fetchall()

    return [
        {
            "id": row["id"],
            "parent_id": row["parent_id"],
            "role": row["role"],
            "content": row["content"] or "",
            "created_at": row["created_at"],
        }
        for row in rows
    ]


def switch_active_leaf(message_id: str | None,conversation_id: str = DEFAULT_CONVERSATION_ID,)->None:
    """
    Switch the active branch to an existing message.

    Passing None resets the current branch to empty.
    """
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        if message_id is not None:
            row=conn.execute(
                """
                SELECT id
                FROM messages
                WHERE id = ?
                  AND conversation_id = ?
                """,
                (message_id, conversation_id),
            ).fetchone()

            if row is None:
                raise ValueError(f"message does not exist: {message_id}")
            
        set_active_leaf_id(conn, message_id, conversation_id)



def pop_last_user_message(conversation_id: str = DEFAULT_CONVERSATION_ID,) -> None:
    """
    Remove active leaf only if it is a user message.

    This is mainly for rollback when the LLM call fails after saving user input.
    """
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        active_leaf_id = get_active_leaf_id(conn, conversation_id)

        if active_leaf_id is None:
            return

        row = conn.execute(
            """
            SELECT id, parent_id, role
            FROM messages
            WHERE id = ?
              AND conversation_id = ?
            """,
            (active_leaf_id, conversation_id),
        ).fetchone()

        if row is None:
            set_active_leaf_id(conn, None, conversation_id)
            return

        if row["role"] != "user":
            return

        set_active_leaf_id(conn, row["parent_id"], conversation_id)

        conn.execute(
            "DELETE FROM messages WHERE id = ?",
            (row["id"],),
        )


def clear_messages(conversation_id: str = DEFAULT_CONVERSATION_ID,) -> None:
    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        set_active_leaf_id(conn, None, conversation_id)

        conn.execute(
            """
            DELETE FROM messages
            WHERE conversation_id = ?
            """,
            (conversation_id,),
        )

        
def delete_conversation(conversation_id: str) -> None:
    if conversation_id == DEFAULT_CONVERSATION_ID:
        raise ValueError("default conversation cannot be deleted")

    with get_conn() as conn:
        ensure_conversation(conn, conversation_id)

        conn.execute(
            """
            DELETE FROM conversations
            WHERE id = ?
            """,
            (conversation_id,),
        )