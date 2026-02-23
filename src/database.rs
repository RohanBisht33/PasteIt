use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use chrono::Utc;
use anyhow::{anyhow, Context};
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: Vec<u8>,
    pub thumbnail: Option<Vec<u8>>,
    pub entry_type: String,
    pub content_hash: String,
    pub timestamp: String,
    pub pinned: bool,
}

impl Database {
    pub fn new() -> anyhow::Result<Self> {
        let db_path = "clipboard_history.db";
        let conn = Connection::open(db_path).context("Failed to open database")?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY,
                content BLOB NOT NULL,
                thumbnail BLOB,
                type TEXT NOT NULL,
                content_hash TEXT UNIQUE NOT NULL,
                timestamp DATETIME NOT NULL,
                pinned BOOLEAN NOT NULL DEFAULT 0
            )",
            [],
        ).context("Failed to create table")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_content_hash ON entries(content_hash)",
            [],
        ).context("Failed to create index")?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp_pinned ON entries(timestamp, pinned)",
            [],
        ).context("Failed to create index")?;

        Ok(Database { conn: Mutex::new(conn) })
    }

    pub fn add_entry(&self, content: &[u8], thumbnail: Option<&[u8]>, entry_type: &str) -> anyhow::Result<()> {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hasher.update(entry_type.as_bytes());
        let hash = hex::encode(hasher.finalize());

        let timestamp = Utc::now().to_rfc3339();

        // Use INSERT OR REPLACE or handle duplication
        // Requirements say deduplicate by SHA256(content + type)
        // If it exists, we update the timestamp to bring it to top
        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        let result = conn.execute(
            "INSERT INTO entries (content, thumbnail, type, content_hash, timestamp, pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)
             ON CONFLICT(content_hash) DO UPDATE SET timestamp = excluded.timestamp",
            params![content, thumbnail, entry_type, hash, timestamp],
        );

        match result {
            Ok(_) => {
                drop(conn); // Release lock before cleanup to avoid deadlock risk though internal cleanup is same conn
                self.cleanup()?;
                Ok(())
            },
            Err(e) => Err(anyhow!("Failed to insert entry: {}", e)),
        }
    }

    pub fn get_history(&self, search: Option<&str>) -> anyhow::Result<Vec<ClipboardEntry>> {
        let mut query = "SELECT id, content, thumbnail, type, content_hash, timestamp, pinned FROM entries".to_string();
        let mut params_vec: Vec<String> = Vec::new();

        if let Some(s) = search {
            query.push_str(" WHERE content LIKE ?1");
            params_vec.push(format!("%{}%", s));
        }
        
        query.push_str(" ORDER BY timestamp DESC LIMIT 500");

        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        let mut stmt = conn.prepare(&query)?;
        
        let entries = stmt.query_map(params_vec.iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>().as_slice(), |row| {
            Ok(ClipboardEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                thumbnail: row.get(2)?,
                entry_type: row.get(3)?,
                content_hash: row.get(4)?,
                timestamp: row.get(5)?,
                pinned: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry?);
        }
        Ok(result)
    }

    pub fn toggle_pin(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        conn.execute(
            "UPDATE entries SET pinned = NOT pinned WHERE id = ?1",
            params![id],
        ).context("Failed to toggle pin")?;
        Ok(())
    }

    pub fn delete_entry(&self, id: i64) -> anyhow::Result<()> {
        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        conn.execute(
            "DELETE FROM entries WHERE id = ?1",
            params![id],
        ).context("Failed to delete entry")?;
        Ok(())
    }

    pub fn clear_history(&self) -> anyhow::Result<()> {
        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        conn.execute(
            "DELETE FROM entries WHERE pinned = 0",
            [],
        ).context("Failed to clear history")?;
        Ok(())
    }

    fn cleanup(&self) -> anyhow::Result<()> {
        // User requested cleanup logic: Delete unpinned beyond 500 limit
        let conn = self.conn.lock().map_err(|_| anyhow!("Failed to lock DB"))?;
        conn.execute(
            "DELETE FROM entries
             WHERE pinned = 0
             AND id NOT IN (
                 SELECT id FROM entries
                 ORDER BY timestamp DESC
                 LIMIT 500
             )",
            [],
        ).context("Failed to cleanup database")?;
        Ok(())
    }
}
