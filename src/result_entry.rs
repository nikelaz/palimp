use std::error::Error;
use crate::database::Database;
use rusqlite::params;

pub struct ResultEntry {
    pub id: Option<i64>,
    pub page_id: i64, 
    pub selector: String,
    pub count: u32,
}

impl ResultEntry {
    pub fn new(id: Option<i64>, page_id: i64, selector: &str, count: u32) -> Self {
        Self {
            id,
            page_id,
            selector: selector.to_string(),
            count,
        }
    }

    pub fn sync(&mut self, database: &mut Database) -> Result<(), Box<dyn Error>> {
        match self.id {
            Some(existing_id) => {
                database.conn.execute(
                    "UPDATE results SET selector = ?1, count = ?2 WHERE id = ?3",
                    params![self.selector, self.count, existing_id],
                )?;
            }
            None => {
                database.conn.execute(
                    "INSERT INTO results (page_id, selector, count) VALUES (?1, ?2, ?3)",
                    params![self.page_id, self.selector, self.count],
                )?;
                self.id = Some(database.conn.last_insert_rowid());
            }
        }
        Ok(())
    }

    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, page_id, selector, count FROM results WHERE id = ?1";

        let entry = database.conn.query_row(sql, params![id], |row| {
            Ok(ResultEntry {
                id: Some(row.get(0)?),
                page_id: row.get(1)?,
                selector: row.get(2)?,
                count: row.get(3)?, // rusqlite converts SQLite INTEGER to u32 automatically
            })
        })?;

        Ok(entry)
    }
}
