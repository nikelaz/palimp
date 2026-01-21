use palimp_core::{Application};
use palimp_core::crawl::Crawl;
use palimp_core::query::Query;
use palimp_core::result_entry::ResultEntry;

async fn create_test_app() -> Application {
    // Use in-memory database for testing
    Application::new(":memory:").expect("Failed to create application with in-memory DB")
}

#[tokio::test]
async fn test_site_lifecycle() {
    let app = create_test_app().await;

    // 1. Create a new site
    app.new_site("example.com", "https://example.com/sitemap.xml")
        .await
        .expect("Failed to create site");

    // 2. List sites and verify
    let sites = app.list_sites().await.expect("Failed to list sites");
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].domain, "example.com");
    assert_eq!(sites[0].sitemap_url, "https://example.com/sitemap.xml");

    // 3. Delete site
    let site_id = sites[0].id.expect("Site ID should be present");
    app.delete_site(site_id).await.expect("Failed to delete site");

    // 4. Verify deletion
    let sites_after = app.list_sites().await.expect("Failed to list sites");
    assert_eq!(sites_after.len(), 0);
}

#[tokio::test]
async fn test_crawl_lifecycle() {
    let app = create_test_app().await;

    // Setup: Create a site first
    app.new_site("example.com", "https://example.com/sitemap.xml").await.unwrap();
    let sites = app.list_sites().await.unwrap();
    let site_id = sites[0].id.unwrap();

    // Manually create a crawl (since new_crawl requires network/mocking)
    // We access the internal DB to simulate a crawl being added
    {
        let mut db_lock = app.db.lock().await;
        let mut crawl = Crawl::new(None, site_id);
        crawl.sync(&mut db_lock).expect("Failed to sync manual crawl");
    }

    // 1. List crawls
    let crawls = app.list_crawls().await.expect("Failed to list crawls");
    assert_eq!(crawls.len(), 1);
    assert_eq!(crawls[0].site_id, site_id);

    // 2. Delete crawl
    let crawl_id = crawls[0].id.expect("Crawl ID should be present");
    app.delete_crawl(crawl_id).await.expect("Failed to delete crawl");

    // 3. Verify deletion
    let crawls_after = app.list_crawls().await.expect("Failed to list crawls");
    assert_eq!(crawls_after.len(), 0);
}

#[tokio::test]
async fn test_query_lifecycle() {
    let app = create_test_app().await;

    // Setup: Site -> Crawl
    app.new_site("test.com", "sitemap").await.unwrap();
    let site_id = app.list_sites().await.unwrap()[0].id.unwrap();
    
    let crawl_id = {
        let mut db = app.db.lock().await;
        let mut crawl = Crawl::new(None, site_id);
        crawl.sync(&mut db).unwrap();
        crawl.id.unwrap()
    };

    // Manually create a Query
    {
        let mut db = app.db.lock().await;
        let mut query = Query::new(None, crawl_id, "div > h1");
        query.sync(&mut db).expect("Failed to sync query");
    }

    // 1. List queries
    let queries = app.list_queries().await.expect("Failed to list queries");
    assert_eq!(queries.len(), 1);
    assert_eq!(queries[0].selector, "div > h1");
    assert_eq!(queries[0].crawl_id, crawl_id);

    // 2. Delete query
    let query_id = queries[0].id.unwrap();
    app.delete_query(query_id).await.expect("Failed to delete query");

    // 3. Verify deletion
    let queries_after = app.list_queries().await.expect("Failed to list queries");
    assert_eq!(queries_after.len(), 0);
}

#[tokio::test]
async fn test_result_lifecycle() {
    let app = create_test_app().await;

    // Setup: Site -> Crawl -> Page (Mocking Page creation requires manual DB insert or accessing Page struct)
    // We need a Page to link a ResultEntry to it.
    // Page struct is public, but let's see if we can use it.
    
    app.new_site("test.com", "sitemap").await.unwrap();
    let site_id = app.list_sites().await.unwrap()[0].id.unwrap();

    let crawl_id = {
        let mut db = app.db.lock().await;
        let mut crawl = Crawl::new(None, site_id);
        crawl.sync(&mut db).unwrap();
        crawl.id.unwrap()
    };
    
    
    // We need to insert a page manually.
    // Page::new(...) returns a Page object, but Page::sync(...) inserts it.
    // check palimp-core/src/page.rs to see if Page::new is usable here (it parses HTML).
    // Page::new(url, final_url, html, id)
    // We can pass empty html.
    
    use palimp_core::page::Page;
    let page_id = {
        let mut db = app.db.lock().await;
        // Mock simple HTML
        let html = "<html><body><h1>Hello</h1></body></html>";
        // Ensure we pass Some(crawl_id)
        let page = Page::new("http://test.com", "http://test.com", html, Some(crawl_id)).expect("Failed to create page");
        page.sync(&mut db).expect("Failed to sync page");
        db.conn.last_insert_rowid()
    };

    // 1. Manually create a ResultEntry linked to the page
    {
        let mut db = app.db.lock().await;
        let mut entry = ResultEntry::new(None, page_id, "h1", 1);
        entry.sync(&mut db).expect("Failed to sync result entry");
    }

    // 2. List results
    let results = app.list_results().await.expect("Failed to list results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].selector, "h1");
    assert_eq!(results[0].page_id, page_id);
    assert_eq!(results[0].count, 1);

    // 3. Delete result
    let result_id = results[0].id.unwrap();
    app.delete_result(result_id).await.expect("Failed to delete result");

    // 4. Verify deletion
    let results_after = app.list_results().await.expect("Failed to list results");
    assert_eq!(results_after.len(), 0);
}

