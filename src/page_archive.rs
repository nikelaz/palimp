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
    pub fn fetch(id: i64, database: &Database) -> Result<Self, Box<dyn Error>> {
        let sql = "SELECT id, url, final_url, html_content, crawl_id FROM pages WHERE id = ?1";
        
        database.conn.query_row(sql, params![id], |row| {
            Ok(PageArchive {
                id: row.get(0)?,
                url: row.get(1)?,
                final_url: row.get(2)?,
                html_content: row.get(3)?,
                crawl_id: row.get(4)?,
            })
        }).map_err(|e| e.into())
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
