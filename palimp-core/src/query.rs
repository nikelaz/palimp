use crate::database::Database;
use rusqlite::params;
use std::error::Error;

pub struct Query {
    pub id: Option<i64>,
    pub crawl_id: i64,
    pub selector: String,
}

impl Query {
    pub fn new(id: Option<i64>, crawl_id: i64, selector: &str) -> Self {
        Self {
            id,
            crawl_id,
            selector: selector.to_string(),
        }
    }

    pub fn sync(&mut self, database: &mut Database) -> Result<(), Box<dyn Error>> {
        match self.id {
            Some(existing_id) => {
                database.conn.execute(
                    "UPDATE queries SET crawl_id = ?1, selector = ?2 WHERE id = ?3",
                    params![self.crawl_id, self.selector, existing_id],
                )?;
            }
            None => {
                database.conn.execute(
                    "INSERT INTO queries (crawl_id, selector) VALUES (?1, ?2)",
                    params![self.crawl_id, self.selector],
                )?;
                self.id = Some(database.conn.last_insert_rowid());
            }
        }
        Ok(())
    }

    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, crawl_id, selector FROM queries WHERE id = ?1";

        database
            .conn
            .query_row(sql, params![id], |row| {
                Ok(Query {
                    id: Some(row.get(0)?),
                    crawl_id: row.get(1)?,
                    selector: row.get(2)?,
                })
            })
            .map_err(|e| e.into())
    }

    pub fn fetch_all(database: &Database) -> Result<Vec<Self>, Box<dyn Error>> {
        let mut stmt = database
            .conn
            .prepare("SELECT id, crawl_id, selector FROM queries")?;

        let query_iter = stmt.query_map([], |row| {
            Ok(Query {
                id: Some(row.get(0)?),
                crawl_id: row.get(1)?,
                selector: row.get(2)?,
            })
        })?;

        let mut queries = Vec::new();
        for q in query_iter {
            queries.push(q?);
        }

        Ok(queries)
    }

    pub fn delete(id: i64, database: &Database) -> Result<(), Box<dyn Error>> {
        database
            .conn
            .execute("DELETE FROM queries WHERE id = ?1", params![id])?;
        Ok(())
    }
}
