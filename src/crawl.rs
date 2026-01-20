use std::error::Error;
use crate::database::Database;
use rusqlite::params;

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
                
                let (new_id, time): (i64, String) = database.conn.query_row(
                    sql,
                    params![self.site_id],
                    |row| Ok((row.get(0)?, row.get(1)?))
                )?;

                self.id = Some(new_id);
                self.started_at = Some(time);
            }
        }
        Ok(())
    }

    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, site_id, started_at FROM crawls WHERE id = ?1";

        database.conn.query_row(sql, params![id], |row| {
            Ok(Crawl {
                id: Some(row.get(0)?),
                site_id: row.get(1)?,
                started_at: Some(row.get(2)?),
            })
        }).map_err(|e| e.into())
    }
}
