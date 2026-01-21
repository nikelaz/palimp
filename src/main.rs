use http_client::HTTPClient;
use page::Page;
use sitemap::Sitemap;
use std::error::Error;
use database::Database;
use site::Site;
use crawl::Crawl;
use query::Query;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::stream::{self, StreamExt};
use page_archive::PageArchive;
use result_entry::ResultEntry;

mod http_client;
mod page;
mod page_archive;
mod sitemap;
mod crawl;
mod site;
mod database;
mod result_entry;
mod query;

pub async fn new_site(domain: &str, sitemap_url: &str, mut db: &mut Database) -> Result<(), Box<dyn Error>> {
    let mut site = Site::new(None, domain, sitemap_url);

    site.sync(&mut db)
        .map_err(|err| format!("Could not create site in the database: {}", err))?;

    Ok(())
}

pub async fn list_sites(db: &Database) -> Result<Vec<Site>, Box<dyn Error>> {
    Site::fetch_all(db)
}

pub async fn delete_site(site_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Site::delete(site_id, db)
}

pub async fn list_crawls(db: &Database) -> Result<Vec<Crawl>, Box<dyn Error>> {
    Crawl::fetch_all(db)
}

pub async fn delete_crawl(crawl_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Crawl::delete(crawl_id, db)
}

pub async fn list_queries(db: &Database) -> Result<Vec<Query>, Box<dyn Error>> {
    Query::fetch_all(db)
}

pub async fn delete_query(query_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Query::delete(query_id, db)
}

pub async fn list_results(db: &Database) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    ResultEntry::fetch_all(db)
}

pub async fn delete_result(result_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    ResultEntry::delete(result_id, db)
}

pub enum CrawlResult {
    PageSucceeded(String),
    PageFailed(String, String),
}

pub async fn new_crawl<F>(
    site_id: i64, 
    db: Arc<Mutex<Database>>,
    http_client: &HTTPClient, 
    max_concurrent: usize,
    on_update: F
) -> Result<(), Box<dyn Error>> 
where 
    F: Fn(CrawlResult) + Send + Sync + 'static 
{
    let site = {
        let db_lock = db.lock().await;
        Site::fetch(site_id, &*db_lock)
            .map_err(|e| format!("DB Error: {}", e))?
    };

    let sitemap_content = http_client.get_sitemap(site.sitemap_url.as_str()).await?;
    let sitemap = Sitemap::new(sitemap_content.as_str())?;

    let on_update = Arc::new(on_update);

    stream::iter(sitemap.urlset.urls)
        .for_each_concurrent(max_concurrent, |url_entry| {
            let url = url_entry.loc;
            let client = http_client.clone();
            let db_clone = Arc::clone(&db);
            let on_update_clone = Arc::clone(&on_update);

            async move {
                let result = process_single_page(&url, db_clone, client).await;

                match result {
                    Ok(_) => on_update_clone(CrawlResult::PageSucceeded(url)),
                    Err(e) => on_update_clone(CrawlResult::PageFailed(url, e.to_string())),
                }
            }
        })
    .await;

    Ok(())
}

async fn process_single_page(
    url: &str, 
    db: Arc<Mutex<Database>>, 
    client: HTTPClient
) -> Result<(), Box<dyn Error>> {
    let (final_url, html) = client.get_html(url).await?;
    let page = Page::new(url, final_url.as_str(), html.as_str(), None)?;

    {
        let mut db_lock = db.lock().await;
        page.sync(&mut *db_lock)?;
    }

    Ok(())
}

async fn query(crawl_id: i64, selector: &str, mut db: &mut Database) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    let pages_archive = PageArchive::fetch_by_crawl_id(crawl_id, &db)?;

    let mut all_results: Vec<ResultEntry> = Vec::new();

    for archive in pages_archive {
        if let Ok(page) = archive.to_page() {
            if let Some(nodes) = page.dom.query_selector(selector) {
                let count_u32 = nodes.count() as u32;
                let mut result_entry = ResultEntry::new(None, archive.id, selector, count_u32);
                let _ = result_entry.sync(&mut db);
                all_results.push(result_entry);
            }
        }
    }

    Ok(all_results)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut db = Database::new("palimp.db")
        .map_err(|err| format!("Database connection failed: {}", err))?;

    let _ = db.seed()
        .map_err(|err| format!("An error occured while seeding the database: {}", err))?;


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
