use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -- Data types --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub sort_order: i32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub vault_id: String,
    pub category: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon_url: Option<String>,
    pub favorite: bool,
    pub trashed: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Summary for list display (no decrypted field values).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySummary {
    pub id: String,
    pub vault_id: String,
    pub category: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon_url: Option<String>,
    pub favorite: bool,
    pub trashed: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub id: String,
    pub entry_id: String,
    pub field_type: String,
    pub label: String,
    pub value: Vec<u8>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldInput {
    pub field_type: String,
    pub label: String,
    pub value: Vec<u8>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryWithFields {
    pub entry: Entry,
    pub fields: Vec<Field>,
}

// -- Vault operations --

pub fn create_vault(conn: &Connection, name: &str, icon: Option<&str>) -> Result<Vault, String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO vaults (id, name, icon, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, 0, ?4, ?5)",
        params![id, name, icon, now, now],
    )
    .map_err(|e| format!("Failed to create vault: {e}"))?;

    Ok(Vault {
        id,
        name: name.to_string(),
        icon: icon.map(|s| s.to_string()),
        sort_order: 0,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn list_vaults(conn: &Connection) -> Result<Vec<Vault>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, icon, sort_order, created_at, updated_at FROM vaults ORDER BY sort_order")
        .map_err(|e| format!("Query failed: {e}"))?;

    let vaults = stmt
        .query_map([], |row| {
            Ok(Vault {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                sort_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(vaults)
}

// -- Entry operations --

pub fn create_entry(
    conn: &Connection,
    vault_id: &str,
    category: &str,
    title: &str,
    subtitle: Option<&str>,
    fields: &[FieldInput],
) -> Result<EntryWithFields, String> {
    let entry_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO entries (id, vault_id, category, title, subtitle, favorite, trashed, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, ?6, ?7)",
        params![entry_id, vault_id, category, title, subtitle, now, now],
    )
    .map_err(|e| format!("Failed to create entry: {e}"))?;

    let mut saved_fields = Vec::new();
    for f in fields {
        let field_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO fields (id, entry_id, field_type, label, value, sort_order) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![field_id, entry_id, f.field_type, f.label, f.value, f.sort_order],
        )
        .map_err(|e| format!("Failed to create field: {e}"))?;

        saved_fields.push(Field {
            id: field_id,
            entry_id: entry_id.clone(),
            field_type: f.field_type.clone(),
            label: f.label.clone(),
            value: f.value.clone(),
            sort_order: f.sort_order,
        });
    }

    let entry = Entry {
        id: entry_id,
        vault_id: vault_id.to_string(),
        category: category.to_string(),
        title: title.to_string(),
        subtitle: subtitle.map(|s| s.to_string()),
        icon_url: None,
        favorite: false,
        trashed: false,
        created_at: now.clone(),
        updated_at: now,
    };

    // Sync FTS index
    let _ = super::search::index_entry(conn, &entry.id, &entry.title, entry.subtitle.as_deref());

    Ok(EntryWithFields {
        entry,
        fields: saved_fields,
    })
}

pub fn get_entry(conn: &Connection, entry_id: &str) -> Result<EntryWithFields, String> {
    let entry = conn
        .query_row(
            "SELECT id, vault_id, category, title, subtitle, icon_url, favorite, trashed, created_at, updated_at \
             FROM entries WHERE id = ?1",
            params![entry_id],
            |row| {
                Ok(Entry {
                    id: row.get(0)?,
                    vault_id: row.get(1)?,
                    category: row.get(2)?,
                    title: row.get(3)?,
                    subtitle: row.get(4)?,
                    icon_url: row.get(5)?,
                    favorite: row.get::<_, i32>(6)? != 0,
                    trashed: row.get::<_, i32>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| format!("Entry not found: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, entry_id, field_type, label, value, sort_order \
             FROM fields WHERE entry_id = ?1 ORDER BY sort_order",
        )
        .map_err(|e| format!("Query failed: {e}"))?;

    let fields = stmt
        .query_map(params![entry_id], |row| {
            Ok(Field {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                field_type: row.get(2)?,
                label: row.get(3)?,
                value: row.get(4)?,
                sort_order: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(EntryWithFields { entry, fields })
}

pub fn list_entries(
    conn: &Connection,
    vault_id: Option<&str>,
    category: Option<&str>,
    trashed: bool,
) -> Result<Vec<EntrySummary>, String> {
    let mut sql = String::from(
        "SELECT id, vault_id, category, title, subtitle, icon_url, favorite, trashed, updated_at \
         FROM entries WHERE trashed = ?1",
    );
    let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> =
        vec![Box::new(trashed as i32)];

    if let Some(vid) = vault_id {
        sql.push_str(" AND vault_id = ?2");
        param_values.push(Box::new(vid.to_string()));
    }
    if let Some(cat) = category {
        let idx = param_values.len() + 1;
        sql.push_str(&format!(" AND category = ?{idx}"));
        param_values.push(Box::new(cat.to_string()));
    }
    sql.push_str(" ORDER BY updated_at DESC");

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Query failed: {e}"))?;
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let entries = stmt
        .query_map(params_ref.as_slice(), |row| {
            Ok(EntrySummary {
                id: row.get(0)?,
                vault_id: row.get(1)?,
                category: row.get(2)?,
                title: row.get(3)?,
                subtitle: row.get(4)?,
                icon_url: row.get(5)?,
                favorite: row.get::<_, i32>(6)? != 0,
                trashed: row.get::<_, i32>(7)? != 0,
                updated_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(entries)
}

pub fn update_entry(
    conn: &Connection,
    entry_id: &str,
    title: Option<&str>,
    subtitle: Option<&str>,
    fields: Option<&[FieldInput]>,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();

    if let Some(t) = title {
        conn.execute(
            "UPDATE entries SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![t, now, entry_id],
        )
        .map_err(|e| format!("Update failed: {e}"))?;
    }

    if let Some(s) = subtitle {
        conn.execute(
            "UPDATE entries SET subtitle = ?1, updated_at = ?2 WHERE id = ?3",
            params![s, now, entry_id],
        )
        .map_err(|e| format!("Update failed: {e}"))?;
    }

    if let Some(new_fields) = fields {
        // Delete existing fields and re-insert (simpler than diffing)
        conn.execute("DELETE FROM fields WHERE entry_id = ?1", params![entry_id])
            .map_err(|e| format!("Delete fields failed: {e}"))?;

        for f in new_fields {
            let field_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO fields (id, entry_id, field_type, label, value, sort_order) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![field_id, entry_id, f.field_type, f.label, f.value, f.sort_order],
            )
            .map_err(|e| format!("Insert field failed: {e}"))?;
        }

        conn.execute(
            "UPDATE entries SET updated_at = ?1 WHERE id = ?2",
            params![now, entry_id],
        )
        .map_err(|e| format!("Update timestamp failed: {e}"))?;
    }

    // Sync FTS index with current title/subtitle
    if title.is_some() || subtitle.is_some() {
        let current: (String, Option<String>) = conn
            .query_row(
                "SELECT title, subtitle FROM entries WHERE id = ?1",
                params![entry_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|e| format!("Read entry failed: {e}"))?;
        let _ = super::search::index_entry(conn, entry_id, &current.0, current.1.as_deref());
    }

    Ok(())
}

pub fn trash_entry(conn: &Connection, entry_id: &str) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let rows = conn
        .execute(
            "UPDATE entries SET trashed = 1, updated_at = ?1 WHERE id = ?2",
            params![now, entry_id],
        )
        .map_err(|e| format!("Trash failed: {e}"))?;

    if rows == 0 {
        return Err("Entry not found".to_string());
    }

    // Remove from FTS index
    let _ = super::search::remove_from_index(conn, entry_id);

    Ok(())
}

pub fn toggle_favorite(conn: &Connection, entry_id: &str) -> Result<bool, String> {
    let current: i32 = conn
        .query_row(
            "SELECT favorite FROM entries WHERE id = ?1",
            params![entry_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Entry not found: {e}"))?;

    let new_val = if current == 0 { 1 } else { 0 };
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE entries SET favorite = ?1, updated_at = ?2 WHERE id = ?3",
        params![new_val, now, entry_id],
    )
    .map_err(|e| format!("Toggle failed: {e}"))?;

    Ok(new_val != 0)
}

/// Save old password value to history before updating.
pub fn save_password_history(
    conn: &Connection,
    entry_id: &str,
    encrypted_value: &[u8],
) -> Result<(), String> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO password_history (id, entry_id, value, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, entry_id, encrypted_value, now],
    )
    .map_err(|e| format!("Save history failed: {e}"))?;
    Ok(())
}

/// Get category counts for sidebar display.
pub fn get_category_counts(
    conn: &Connection,
    vault_id: Option<&str>,
) -> Result<Vec<(String, i64)>, String> {
    let (sql, param_values): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(vid) = vault_id {
        (
            "SELECT category, COUNT(*) FROM entries WHERE trashed = 0 AND vault_id = ?1 GROUP BY category".to_string(),
            vec![Box::new(vid.to_string())],
        )
    } else {
        (
            "SELECT category, COUNT(*) FROM entries WHERE trashed = 0 GROUP BY category".to_string(),
            vec![],
        )
    };

    let mut stmt = conn.prepare(&sql).map_err(|e| format!("Query failed: {e}"))?;
    let params_ref: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();

    let counts = stmt
        .query_map(params_ref.as_slice(), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_list_vaults() {
        let conn = setup_db();
        create_vault(&conn, "Personal", None).unwrap();
        create_vault(&conn, "Work", Some("briefcase")).unwrap();
        let vaults = list_vaults(&conn).unwrap();
        assert_eq!(vaults.len(), 2);
        assert_eq!(vaults[0].name, "Personal");
    }

    #[test]
    fn create_and_get_entry() {
        let conn = setup_db();
        let vault = create_vault(&conn, "Personal", None).unwrap();
        let fields = vec![
            FieldInput {
                field_type: "username".to_string(),
                label: "Username".to_string(),
                value: b"john@example.com".to_vec(),
                sort_order: 0,
            },
            FieldInput {
                field_type: "password".to_string(),
                label: "Password".to_string(),
                value: b"encrypted-blob".to_vec(),
                sort_order: 1,
            },
        ];

        let created = create_entry(&conn, &vault.id, "login", "GitHub", Some("john"), &fields).unwrap();
        assert_eq!(created.entry.title, "GitHub");
        assert_eq!(created.fields.len(), 2);

        let fetched = get_entry(&conn, &created.entry.id).unwrap();
        assert_eq!(fetched.entry.title, "GitHub");
        assert_eq!(fetched.fields.len(), 2);
        assert_eq!(fetched.fields[0].value, b"john@example.com");
    }

    #[test]
    fn list_entries_with_filter() {
        let conn = setup_db();
        let vault = create_vault(&conn, "Personal", None).unwrap();
        create_entry(&conn, &vault.id, "login", "Site A", None, &[]).unwrap();
        create_entry(&conn, &vault.id, "note", "My Note", None, &[]).unwrap();
        create_entry(&conn, &vault.id, "login", "Site B", None, &[]).unwrap();

        let all = list_entries(&conn, None, None, false).unwrap();
        assert_eq!(all.len(), 3);

        let logins = list_entries(&conn, None, Some("login"), false).unwrap();
        assert_eq!(logins.len(), 2);

        let notes = list_entries(&conn, None, Some("note"), false).unwrap();
        assert_eq!(notes.len(), 1);
    }

    #[test]
    fn trash_entry_soft_delete() {
        let conn = setup_db();
        let vault = create_vault(&conn, "Personal", None).unwrap();
        let entry = create_entry(&conn, &vault.id, "login", "Test", None, &[]).unwrap();

        trash_entry(&conn, &entry.entry.id).unwrap();

        let active = list_entries(&conn, None, None, false).unwrap();
        assert_eq!(active.len(), 0);

        let trashed = list_entries(&conn, None, None, true).unwrap();
        assert_eq!(trashed.len(), 1);
    }

    #[test]
    fn category_counts() {
        let conn = setup_db();
        let vault = create_vault(&conn, "Personal", None).unwrap();
        create_entry(&conn, &vault.id, "login", "A", None, &[]).unwrap();
        create_entry(&conn, &vault.id, "login", "B", None, &[]).unwrap();
        create_entry(&conn, &vault.id, "note", "C", None, &[]).unwrap();

        let counts = get_category_counts(&conn, None).unwrap();
        let login_count = counts.iter().find(|(c, _)| c == "login").map(|(_, n)| *n).unwrap_or(0);
        let note_count = counts.iter().find(|(c, _)| c == "note").map(|(_, n)| *n).unwrap_or(0);
        assert_eq!(login_count, 2);
        assert_eq!(note_count, 1);
    }
}
