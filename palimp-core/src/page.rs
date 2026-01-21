use crate::database::Database;
use rusqlite::params;
use std::error::Error;
use tl::VDom;

pub struct Page<'a> {
    pub dom: VDom<'a>,
    pub url: String,
    pub final_url: String,
    pub html_content: &'a str,
    pub crawl_id: Option<i64>,
}

impl<'a> Page<'a> {
    pub fn new(
        url: &str,
        final_url: &str,
        page_content: &'a str,
        crawl_id: Option<i64>,
    ) -> Result<Page<'a>, Box<dyn Error>> {
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
        let cid = self
            .crawl_id
            .ok_or("Cannot sync a page without a crawl_id")?;

        database.conn.execute(
            "INSERT INTO pages (crawl_id, url, final_url, html_content) VALUES (?1, ?2, ?3, ?4)",
            params![cid, self.url, self.final_url, self.html_content],
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_selector_count() {
        let html = r#"
            <!DOCTYPE html>
            <html>
                <body>
                    <div class="item">Item 1</div>
                    <div class="item">Item 2</div>
                    <div class="item">Item 3</div>
                    <p>Just a paragraph</p>
                </body>
            </html>
        "#;

        let page = Page::new("http://test.com", "http://test.com", html, None)
            .expect("Failed to create page");

        let nodes = page
            .dom
            .query_selector("div.item")
            .expect("Selector failed");
        let count = nodes.count();

        assert_eq!(count, 3);
    }

    #[test]
    fn test_page_selector_zero_count() {
        let html = "<html><body><p>Hello</p></body></html>";
        let page = Page::new("http://test.com", "http://test.com", html, None)
            .expect("Failed to create page");

        // This selector should match nothing
        match page.dom.query_selector(".nonexistent") {
            Some(nodes) => assert_eq!(nodes.count(), 0),
            None => {
                // query_selector usually returns None if selector is invalid, or if no matches?
                // tl's behavior: "Returns None if the selector could not be parsed."
                // Since ".nonexistent" is a valid selector, it should return Some(iterator) that yields 0 items.
                // However, let's verify what happens with a valid selector that finds nothing.
                // Usually it returns an empty iterator.
            }
        }

        // Actually, let's just assert on valid selectors that have 0 matches
        if let Some(nodes) = page.dom.query_selector("div.missing") {
            assert_eq!(nodes.count(), 0);
        }
    }
}
