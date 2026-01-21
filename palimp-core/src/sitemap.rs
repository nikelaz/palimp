use quick_xml::de::from_str;
use serde::{Deserialize};
use std::error::Error;

fn parse_sitemap(xml_content: &str) -> Result<UrlSet, Box<dyn std::error::Error>> {
    let sitemap: UrlSet = from_str(xml_content)?;
    Ok(sitemap)
}

#[derive(Debug, Deserialize)]
pub struct UrlSet {
    #[serde(rename = "url")]
    pub urls: Vec<SitemapUrl>,
}

#[derive(Debug, Deserialize)]
pub struct SitemapUrl {
    pub loc: String,
}

pub struct Sitemap {
    pub urlset: UrlSet,
}

impl Sitemap {
    pub fn new(sitemap_content: &str) -> Result<Sitemap, Box<dyn Error>> {
        let urlset = parse_sitemap(sitemap_content)?;

        Ok(Sitemap { urlset: urlset })
    }
}
