pub mod http_client;
pub mod page;
pub mod page_archive;
pub mod sitemap;
pub mod crawl;
pub mod site;
pub mod database;
pub mod result_entry;
pub mod query;

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
use rusqlite::params;

pub struct Application {
    pub db: Arc<Mutex<Database>>,
    pub http_client: HTTPClient,
}

impl Application {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn Error>> {
        let db = Database::new(db_path)?;
        db.seed()?;
        let http_client = HTTPClient::new()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            http_client,
        })
    }

    pub async fn new_site(&self, domain: &str, sitemap_url: &str) -> Result<(), Box<dyn Error>> {
        let mut db = self.db.lock().await;
        new_site(domain, sitemap_url, &mut db).await
    }

    pub async fn list_sites(&self) -> Result<Vec<Site>, Box<dyn Error>> {
        let db = self.db.lock().await;
        list_sites(&db).await
    }

    pub async fn delete_site(&self, site_id: i64) -> Result<(), Box<dyn Error>> {
        let db = self.db.lock().await;
        delete_site(site_id, &db).await
    }

    pub async fn list_crawls(&self) -> Result<Vec<Crawl>, Box<dyn Error>> {
        let db = self.db.lock().await;
        list_crawls(&db).await
    }

    pub async fn delete_crawl(&self, crawl_id: i64) -> Result<(), Box<dyn Error>> {
        let db = self.db.lock().await;
        delete_crawl(crawl_id, &db).await
    }

    pub async fn list_queries(&self) -> Result<Vec<Query>, Box<dyn Error>> {
        let db = self.db.lock().await;
        list_queries(&db).await
    }

    pub async fn delete_query(&self, query_id: i64) -> Result<(), Box<dyn Error>> {
        let db = self.db.lock().await;
        delete_query(query_id, &db).await
    }

    pub async fn list_results(&self) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
        let db = self.db.lock().await;
        list_results(&db).await
    }
    
    pub async fn list_results_for_query(&self, query_id: i64) -> Result<Vec<(ResultEntry, String)>, Box<dyn Error>> {
        let db = self.db.lock().await;
        
        let query = Query::fetch(query_id, &db)?;
        
        let results = ResultEntry::fetch_by_crawl_and_selector(query.crawl_id, &query.selector, &db)?;
        
        let mut enriched_results = Vec::new();
        for res in results { 
             let page_url: String = db.conn.query_row(
                "SELECT url FROM pages WHERE id = ?1",
                params![res.page_id],
                |row| row.get(0)
             )?;
             
             enriched_results.push((res, page_url));
        }
        
        Ok(enriched_results)
    }

    pub async fn delete_result(&self, result_id: i64) -> Result<(), Box<dyn Error>> {
        let db = self.db.lock().await;
        delete_result(result_id, &db).await
    }

    pub async fn new_crawl<F>(&self, site_id: i64, max_concurrent: usize, on_update: F) -> Result<(), Box<dyn Error>>
    where
        F: Fn(CrawlResult) + Send + Sync + 'static,
    {
        new_crawl(site_id, self.db.clone(), &self.http_client, max_concurrent, on_update).await
    }

    pub async fn query(&self, crawl_id: i64, selector: &str) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
        let mut db = self.db.lock().await;
        
        // Save the query definition
        let mut q = Query::new(None, crawl_id, selector);
        q.sync(&mut db)?;

        query(crawl_id, selector, &mut db).await
    }
}


async fn new_site(domain: &str, sitemap_url: &str, mut db: &mut Database) -> Result<(), Box<dyn Error>> {
    let mut site = Site::new(None, domain, sitemap_url);

    site.sync(&mut db)
        .map_err(|err| format!("Could not create site in the database: {}", err))?;

    Ok(())
}

async fn list_sites(db: &Database) -> Result<Vec<Site>, Box<dyn Error>> {
    Site::fetch_all(db)
}

async fn delete_site(site_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Site::delete(site_id, db)
}

async fn list_crawls(db: &Database) -> Result<Vec<Crawl>, Box<dyn Error>> {
    Crawl::fetch_all(db)
}

async fn delete_crawl(crawl_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Crawl::delete(crawl_id, db)
}

async fn list_queries(db: &Database) -> Result<Vec<Query>, Box<dyn Error>> {
    Query::fetch_all(db)
}

async fn delete_query(query_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    Query::delete(query_id, db)
}

async fn list_results(db: &Database) -> Result<Vec<ResultEntry>, Box<dyn Error>> {
    ResultEntry::fetch_all(db)
}

async fn delete_result(result_id: i64, db: &Database) -> Result<(), Box<dyn Error>> {
    ResultEntry::delete(result_id, db)
}

pub enum CrawlResult {
    PageSucceeded(String),
    PageFailed(String, String),
}

async fn new_crawl<F>(
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

    // Create and sync the crawl first to generate its ID
    let crawl_id = {
        let mut db_lock = db.lock().await;
        let mut crawl = Crawl::new(None, site_id);
        crawl.sync(&mut *db_lock)?;
        crawl.id.ok_or("Failed to get crawl ID after sync")?
    };

    let on_update = Arc::new(on_update);

    stream::iter(sitemap.urlset.urls)
        .for_each_concurrent(max_concurrent, |url_entry| {
            let url = url_entry.loc;
            let client = http_client.clone();
            let db_clone = Arc::clone(&db);
            let on_update_clone = Arc::clone(&on_update);
            let crawl_id = crawl_id; // Capture crawl_id for the async block

            async move {
                let result = process_single_page(&url, crawl_id, db_clone, client).await;

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
    crawl_id: i64,
    db: Arc<Mutex<Database>>, 
    client: HTTPClient
) -> Result<(), Box<dyn Error>> {
    let (final_url, html) = client.get_html(url).await?;
    let page = Page::new(url, final_url.as_str(), html.as_str(), Some(crawl_id))?;

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
                if count_u32 > 0 {
                    let mut result_entry = ResultEntry::new(None, archive.id, selector, count_u32);
                    let _ = result_entry.sync(&mut db);
                    all_results.push(result_entry);
                }
            }
        }
    }

    Ok(all_results)
}
