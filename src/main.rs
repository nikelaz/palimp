use http_client::HTTPClient;
use page::Page;
use sitemap::Sitemap;

mod http_client;
mod page;
mod sitemap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new()?;

    let url = "https://www.rust-lang.org";

    let (final_url, response_text) = http_client.get_html(url).await?;

    println!("Final URL: {}", final_url);

    let sitemap_url = "https://www.progress.com/sitemap.xml";

    let sitemap_content = http_client.get_sitemap(sitemap_url).await?;

    let sitemap = Sitemap::new(sitemap_content.as_str())
        .map_err(|err| format!("An error occured while parsing sitemap {}:\n{}", sitemap_url, err))?;

    println!("URLs in sitemap: {}", sitemap.urlset.urls.len());

    let page = Page::new(url, final_url.as_str(), response_text.as_str())
        .map_err(|err| format!("Could not parse response text as HTML for {}, \n{}", url, err))?;

    let selector = "h2";

    let results = page.dom.query_selector(selector);

    match results {
        Some(res) => println!("Found {} elements for selector {}", res.count(), selector),
        None => println!("No results found"),
    }

    Ok(())
}
