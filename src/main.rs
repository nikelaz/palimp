use http_client::HTTPClient;
use page::Page;
use sitemap::Sitemap;
use std::error::Error;
use database::Database;
use site::Site;

mod http_client;
mod page;
mod page_archive;
mod sitemap;
mod crawl;
mod site;
mod database;
mod result_entry;

async fn new_site(domain: &str, sitemap_url: &str, mut db: &mut Database) -> Result<(), Box<dyn Error>> {
    let mut site = Site::new(None, domain, sitemap_url);

    site.sync(&mut db)
        .map_err(|err| format!("Could not create site in the database: {}", err))?;

    /* this should be new_crawl()
    let sitemap_content = http_client.get_sitemap(sitemap_url).await?;

    let sitemap = Sitemap::new(sitemap_content.as_str())
        .map_err(|err| format!("An error occured while parsing sitemap {}:\n{}", site.sitemap_url, err))?;

    for url in sitemap.urlset.urls {
        let url = url.loc;

        let (final_url, response_text) = http_client.get_html(url.as_str()).await?;

        let page = Page::new(url.as_str(), final_url.as_str(), response_text.as_str(), None)
            .map_err(|err| format!("Could not parse response text as HTML for {}, \n{}", url, err))?;

        page.sync(&mut db)
            .map_err(|err| format!("Could not create page in the database: {}", err))?;
    }
    */

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut db = Database::new("palimp.db")
        .map_err(|err| format!("Database connection failed: {}", err))?;

    let _ = db.seed()
        .map_err(|err| format!("An error occured while seeding the database: {}", err))?;

    let http_client = HTTPClient::new()?;

    /*
    let http_client = HTTPClient::new()?;

    let url = "https://www.rust-lang.org";

    let (final_url, response_text) = http_client.get_html(url).await?;

    println!("Final URL: {}", final_url);

    let sitemap_url = "https://www.progress.com/sitemap.xml";

    let sitemap_content = http_client.get_sitemap(sitemap_url).await?;

    let sitemap = Sitemap::new(sitemap_content.as_str())
        .map_err(|err| format!("An error occured while parsing sitemap {}:\n{}", sitemap_url, err))?;

    println!("URLs in sitemap: {}", sitemap.urlset.urls.len());

    let page = Page::new(url, final_url.as_str(), response_text.as_str(), None)
        .map_err(|err| format!("Could not parse response text as HTML for {}, \n{}", url, err))?;

    let selector = "h2";

    let results = page.dom.query_selector(selector);

    match results {
        Some(res) => println!("Found {} elements for selector {}", res.count(), selector),
        None => println!("No results found"),
    }
    */
    Ok(())
}
