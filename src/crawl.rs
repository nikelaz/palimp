use crate::database::Database;
use rusqlite::params;
use std::error::Error;

pub struct Crawl {
    pub id: Option<i64>,
    pub site_id: i64,
    pub started_at: Option<String>,
}

impl Crawl {
    pub fn new(id: Option<i64>, site_id: i64) -> Crawl {
        Crawl {
            id,
            site_id,
            started_at: None,
        }
    }

    pub fn sync(&mut self, database: &mut Database) -> Result<(), Box<dyn Error>> {
        match self.id {
            Some(existing_id) => {
                database.conn.execute(
                    "UPDATE crawls SET site_id = ?1 WHERE id = ?2",
                    params![self.site_id, existing_id],
                )?;
            }
            None => {
                let sql = "INSERT INTO crawls (site_id) VALUES (?1) RETURNING id, started_at";

                let (new_id, time): (i64, String) =
                    database.conn.query_row(sql, params![self.site_id], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })?;

                self.id = Some(new_id);
                self.started_at = Some(time);
            }
        }
        Ok(())
    }

    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, site_id, started_at FROM crawls WHERE id = ?1";

        database
            .conn
            .query_row(sql, params![id], |row| {
                Ok(Crawl {
                    id: Some(row.get(0)?),
                    site_id: row.get(1)?,
                    started_at: Some(row.get(2)?),
                })
            })
            .map_err(|e| e.into())
    }

    pub fn fetch_all(database: &Database) -> Result<Vec<Self>, Box<dyn Error>> {
        let mut stmt = database
            .conn
            .prepare("SELECT id, site_id, started_at FROM crawls")?;

        let crawl_iter = stmt.query_map([], |row| {
            Ok(Crawl {
                id: Some(row.get(0)?),
                site_id: row.get(1)?,
                started_at: Some(row.get(2)?),
            })
        })?;

        let mut crawls = Vec::new();
        for crawl in crawl_iter {
            crawls.push(crawl?);
        }

        Ok(crawls)
    }

    pub fn delete(id: i64, database: &Database) -> Result<(), Box<dyn Error>> {
        database
            .conn
            .execute("DELETE FROM crawls WHERE id = ?1", params![id])?;
        Ok(())
    }
}
