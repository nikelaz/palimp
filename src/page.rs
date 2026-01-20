use std::error::Error;
use tl::VDom;
use crate::database::Database;
use rusqlite::params;

pub struct Page<'a> {
    pub dom: VDom<'a>,
    pub url: String,
    pub final_url: String,
    pub html_content: &'a str,
    pub crawl_id: Option<i64>
}

impl<'a> Page<'a> {
    pub fn new(url: &str, final_url: &str, page_content: &'a str, crawl_id: Option<i64>) -> Result<Page<'a>, Box<dyn Error>> {
        let dom = tl::parse(page_content, tl::ParserOptions::default())?;

        Ok(Page {
            dom: dom,
            url: url.to_string(),
            final_url: final_url.to_string(),
            html_content: page_content,
            crawl_id: crawl_id,
        })
    }

    pub fn sync(&self, database: &mut Database) -> Result<(), Box<dyn Error>> {
        // We ensure there is a crawl_id before saving, otherwise it's an orphan
        let cid = self.crawl_id.ok_or("Cannot sync a page without a crawl_id")?;

        // Pages are typically "Insert Only" for an archive—we don't usually update them.
        // If you want to prevent duplicates in the same crawl, 
        // you'd use ON CONFLICT(crawl_id, url) DO NOTHING.
        database.conn.execute(
            "INSERT INTO pages (crawl_id, url, final_url, html_content) VALUES (?1, ?2, ?3, ?4)",
            params![cid, self.url, self.final_url, self.html_content],
        )?;

        Ok(())
    }
}
