use rusqlite::{params, Connection};

use super::queries::EntrySummary;

/// Search entries by title and subtitle using LIKE (prefix + contains matching).
/// For < 5000 entries this is < 10ms. Upgrade to FTS5 in M3 if needed.
pub fn search_entries(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<EntrySummary>, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(vec![]);
    }

    let pattern = format!("%{trimmed}%");

    let sql = "
        SELECT id, vault_id, category, title, subtitle, icon_url,
               favorite, trashed, updated_at
        FROM entries
        WHERE trashed = 0
          AND (title LIKE ?1 COLLATE NOCASE OR subtitle LIKE ?1 COLLATE NOCASE)
        ORDER BY
            CASE WHEN title LIKE ?2 COLLATE NOCASE THEN 0 ELSE 1 END,
            updated_at DESC
        LIMIT ?3
    ";

    // prefix_pattern gives priority to prefix matches
    let prefix_pattern = format!("{trimmed}%");

    let mut stmt = conn.prepare(sql).map_err(|e| format!("Search failed: {e}"))?;
    let results = stmt
        .query_map(params![pattern, prefix_pattern, limit as i64], |row| {
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
        .map_err(|e| format!("Search failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

/// Get recently used entries (by updated_at descending).
pub fn recent_entries(conn: &Connection, limit: usize) -> Result<Vec<EntrySummary>, String> {
    let sql = "
        SELECT id, vault_id, category, title, subtitle, icon_url,
               favorite, trashed, updated_at
        FROM entries
        WHERE trashed = 0
        ORDER BY updated_at DESC
        LIMIT ?1
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| format!("Recent query failed: {e}"))?;
    let results = stmt
        .query_map(params![limit as i64], |row| {
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
        .map_err(|e| format!("Recent query failed: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(results)
}

// FTS index sync functions — no-op until FTS5 is enabled in M3
pub fn index_entry(_conn: &Connection, _entry_id: &str, _title: &str, _subtitle: Option<&str>) -> Result<(), String> {
    Ok(())
}

pub fn remove_from_index(_conn: &Connection, _entry_id: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{queries, schema};

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        schema::run_migrations(&conn).unwrap();
        conn
    }

    #[test]
    fn search_finds_by_title() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "GitHub", Some("john"), &[]).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "GitLab", Some("alice"), &[]).unwrap();
        queries::create_entry(&conn, &vault.id, "note", "My Note", None, &[]).unwrap();

        let results = search_entries(&conn, "git", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn search_finds_by_subtitle() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "Work Email", Some("alice@company.com"), &[]).unwrap();

        let results = search_entries(&conn, "alice", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Work Email");
    }

    #[test]
    fn search_case_insensitive() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "GitHub", None, &[]).unwrap();

        let results = search_entries(&conn, "GITHUB", 10).unwrap();
        assert_eq!(results.len(), 1);

        let results2 = search_entries(&conn, "github", 10).unwrap();
        assert_eq!(results2.len(), 1);
    }

    #[test]
    fn search_excludes_trashed() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        let entry = queries::create_entry(&conn, &vault.id, "login", "ToTrash", None, &[]).unwrap();
        queries::trash_entry(&conn, &entry.entry.id).unwrap();

        let results = search_entries(&conn, "ToTrash", 10).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let conn = setup_db();
        assert_eq!(search_entries(&conn, "", 10).unwrap().len(), 0);
        assert_eq!(search_entries(&conn, "   ", 10).unwrap().len(), 0);
    }

    #[test]
    fn search_prefix_match_ranked_first() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "MyGitHub", None, &[]).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "GitHub", None, &[]).unwrap();

        let results = search_entries(&conn, "Git", 10).unwrap();
        assert_eq!(results.len(), 2);
        // "GitHub" (prefix match) should come before "MyGitHub" (contains match)
        assert_eq!(results[0].title, "GitHub");
    }

    #[test]
    fn recent_entries_ordered() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "Old", None, &[]).unwrap();
        queries::create_entry(&conn, &vault.id, "login", "New", None, &[]).unwrap();

        let recent = recent_entries(&conn, 5).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].title, "New");
    }

    #[test]
    fn recent_entries_respects_limit() {
        let conn = setup_db();
        let vault = queries::create_vault(&conn, "Personal", None).unwrap();
        for i in 0..10 {
            queries::create_entry(&conn, &vault.id, "login", &format!("Site {i}"), None, &[]).unwrap();
        }

        let recent = recent_entries(&conn, 3).unwrap();
        assert_eq!(recent.len(), 3);
    }
}
