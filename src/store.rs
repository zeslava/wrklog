use std::sync::Mutex;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

pub const TS_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id   INTEGER PRIMARY KEY,
                ts   TEXT NOT NULL,
                text TEXT NOT NULL,
                msg_id INTEGER
            );
            CREATE TABLE IF NOT EXISTS owner (
                id      INTEGER PRIMARY KEY CHECK (id = 1),
                chat_id INTEGER NOT NULL
            );",
        )?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version == 0 {
            // DBs created before msg_id existed already have `entries` without the column.
            let has_msg_id: i64 = conn.query_row(
                "SELECT COUNT(*) FROM pragma_table_info('entries') WHERE name = 'msg_id'",
                [],
                |r| r.get(0),
            )?;
            if has_msg_id == 0 {
                conn.execute_batch("ALTER TABLE entries ADD COLUMN msg_id INTEGER")?;
            }
            conn.execute_batch("PRAGMA user_version = 1")?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn owner(&self) -> Result<Option<i64>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let id = conn
            .query_row("SELECT chat_id FROM owner WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(id)
    }

    /// Binds the first caller as owner; returns the actual owner afterwards.
    pub fn claim_owner(&self, chat_id: i64) -> Result<i64> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT OR IGNORE INTO owner (id, chat_id) VALUES (1, ?1)",
            params![chat_id],
        )?;
        let id = conn.query_row("SELECT chat_id FROM owner WHERE id = 1", [], |r| r.get(0))?;
        Ok(id)
    }

    pub fn add(&self, ts: &str, text: &str, msg_id: i32) -> Result<()> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        conn.execute(
            "INSERT INTO entries (ts, text, msg_id) VALUES (?1, ?2, ?3)",
            params![ts, text, msg_id],
        )?;
        Ok(())
    }

    /// Returns false if no entry was recorded from this message.
    pub fn update(&self, msg_id: i32, text: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let n = conn.execute(
            "UPDATE entries SET text = ?2 WHERE msg_id = ?1",
            params![msg_id, text],
        )?;
        Ok(n > 0)
    }

    /// `ts` uses TS_FORMAT, so lexicographic comparison matches chronological order.
    pub fn since(&self, ts: &str) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().expect("store mutex poisoned");
        let mut stmt = conn.prepare("SELECT ts, text FROM entries WHERE ts >= ?1 ORDER BY ts")?;
        let rows = stmt
            .query_map(params![ts], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}
