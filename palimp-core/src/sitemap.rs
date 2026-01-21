use quick_xml::de::from_str;
use serde::Deserialize;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sitemap() {
        let xml = r#"
            <?xml version="1.0" encoding="UTF-8"?>
            <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
                <url>
                    <loc>https://example.com/</loc>
                </url>
                <url>
                    <loc>https://example.com/about</loc>
                </url>
            </urlset>
        "#;

        let sitemap = Sitemap::new(xml).expect("Failed to parse sitemap");
        assert_eq!(sitemap.urlset.urls.len(), 2);
        assert_eq!(sitemap.urlset.urls[0].loc, "https://example.com/");
        assert_eq!(sitemap.urlset.urls[1].loc, "https://example.com/about");
    }
}
