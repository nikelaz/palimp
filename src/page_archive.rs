use std::error::Error;
use rusqlite::params;
use crate::database::Database;
use crate::page::Page;

pub struct PageArchive {
    pub id: i64,
    pub url: String,
    pub final_url: String,
    pub html_content: String,
    pub crawl_id: i64,
}

impl PageArchive {
    pub fn fetch(id: i64, db: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, url, final_url, html_content, crawl_id FROM pages WHERE id = ?1";
        
        db.conn.query_row(sql, params![id], |row| {
            Ok(PageArchive {
                id: row.get(0)?,
                url: row.get(1)?,
                final_url: row.get(2)?,
                html_content: row.get(3)?,
                crawl_id: row.get(4)?,
            })
        }).map_err(|e| e.into())
    }

    pub fn fetch_by_crawl_id(crawl_id: i64, db: &Database) -> Result<Vec<Self>, Box<dyn Error>> {
        let sql = "SELECT id, url, final_url, html_content, crawl_id FROM pages WHERE crawl_id = ?1";

        let mut stmt = db.conn.prepare(sql)?;

        let rows = stmt.query_map([crawl_id], |row| {
            Ok(PageArchive {
                id: row.get(0)?,
                url: row.get(1)?,
                final_url: row.get(2)?,
                html_content: row.get(3)?,
                crawl_id: row.get(4)?,
            })
        })?;

        let mut results = Vec::new();
        for row_result in rows {
            results.push(row_result?);
        }

        Ok(results)
    }

    pub fn to_page(&self) -> Result<Page<'_>, Box<dyn Error>> {
        Page::new(
            &self.url, 
            &self.final_url, 
            &self.html_content, 
            Some(self.crawl_id)
        )
    }
}
