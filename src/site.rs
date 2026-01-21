use crate::database::Database;
use rusqlite::params;
use std::error::Error;

pub struct Site {
    pub id: Option<i64>,
    pub domain: String,
    pub sitemap_url: String,
}

impl Site {
    pub fn new(id: Option<i64>, domain: &str, sitemap_url: &str) -> Site {
        Site {
            id,
            domain: domain.to_string(),
            sitemap_url: sitemap_url.to_string(),
        }
    }

    pub fn sync(&mut self, database: &mut Database) -> Result<(), Box<dyn Error>> {
        match self.id {
            Some(existing_id) => {
                database.conn.execute(
                    "UPDATE sites SET domain = ?1, sitemap_url = ?2 WHERE id = ?3",
                    params![self.domain, self.sitemap_url, existing_id],
                )?;
                Ok(())
            }
            None => {
                database.conn.execute(
                    "INSERT INTO sites (domain, sitemap_url) VALUES (?1, ?2)",
                    params![self.domain, self.sitemap_url],
                )?;

                self.id = Some(database.conn.last_insert_rowid());
                Ok(())
            }
        }
    }

    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, domain, sitemap_url FROM sites WHERE id = ?1";

        let site = database.conn.query_row(sql, params![id], |row| {
            Ok(Site {
                id: Some(row.get(0)?),
                domain: row.get(1)?,
                sitemap_url: row.get(2)?, // rusqlite handles Option<String> automatically
            })
        })?;

        Ok(site)
    }

    pub fn fetch_all(database: &Database) -> Result<Vec<Site>, Box<dyn Error>> {
        let mut stmt = database
            .conn
            .prepare("SELECT id, domain, sitemap_url FROM sites")?;

        let site_iter = stmt.query_map([], |row| {
            Ok(Site {
                id: Some(row.get(0)?),
                domain: row.get(1)?,
                sitemap_url: row.get(2)?,
            })
        })?;

        let mut sites = Vec::new();
        for site in site_iter {
            sites.push(site?);
        }

        Ok(sites)
    }

    pub fn delete(id: i64, database: &Database) -> Result<(), Box<dyn Error>> {
        database
            .conn
            .execute("DELETE FROM sites WHERE id = ?1", params![id])?;
        Ok(())
    }
}
