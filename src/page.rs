use std::error::Error;
use tl::VDom;

pub struct Page<'a> {
    pub dom: VDom<'a>,
    pub url: String,
    pub final_url: String,
}

impl<'a> Page<'a> {
    pub fn new(url: &str, final_url: &str, page_content: &'a str) -> Result<Page<'a>, Box<dyn Error>> {
        let dom = tl::parse(page_content, tl::ParserOptions::default())?;

        Ok(Page {
            dom: dom,
            url: url.to_string(),
            final_url: final_url.to_string(),
        })
    }
}
