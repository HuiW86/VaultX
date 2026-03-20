use rusqlite::Connection;

/// Run all database migrations.
/// Idempotent: uses IF NOT EXISTS for all tables.
pub fn run_migrations(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(SCHEMA_V1).map_err(|e| format!("Migration failed: {e}"))
}

const SCHEMA_V1: &str = "
-- Vaults
CREATE TABLE IF NOT EXISTS vaults (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    icon        TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- Entries
CREATE TABLE IF NOT EXISTS entries (
    id          TEXT PRIMARY KEY,
    vault_id    TEXT NOT NULL REFERENCES vaults(id),
    category    TEXT NOT NULL CHECK(category IN ('login', 'card', 'note', 'identity', 'ssh_key')),
    title       TEXT NOT NULL,
    subtitle    TEXT,
    icon_url    TEXT,
    favorite    INTEGER NOT NULL DEFAULT 0,
    trashed     INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entries_vault ON entries(vault_id);
CREATE INDEX IF NOT EXISTS idx_entries_category ON entries(category);
CREATE INDEX IF NOT EXISTS idx_entries_trashed ON entries(trashed);

-- Fields (each entry has multiple fields)
-- Sensitive field values (password, hidden, card_number) are AES-256-GCM encrypted BLOBs.
-- Non-sensitive field values (text, username, url, otp) are stored as plaintext TEXT.
CREATE TABLE IF NOT EXISTS fields (
    id          TEXT PRIMARY KEY,
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    field_type  TEXT NOT NULL,
    label       TEXT NOT NULL,
    value       BLOB NOT NULL,
    sort_order  INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_fields_entry ON fields(entry_id);

-- Password history (auto-saved when password field changes)
CREATE TABLE IF NOT EXISTS password_history (
    id          TEXT PRIMARY KEY,
    entry_id    TEXT NOT NULL REFERENCES entries(id) ON DELETE CASCADE,
    value       BLOB NOT NULL,
    created_at  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_pw_history_entry ON password_history(entry_id);

-- Enable foreign keys
PRAGMA foreign_keys = ON;
";

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migrations_run_cleanly() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"vaults".to_string()));
        assert!(tables.contains(&"entries".to_string()));
        assert!(tables.contains(&"fields".to_string()));
        assert!(tables.contains(&"password_history".to_string()));
    }

    #[test]
    fn migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap(); // Should not fail
    }
}
