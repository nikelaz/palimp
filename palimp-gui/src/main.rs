use palimp_core::{Application, CrawlResult};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::mpsc;

slint::include_modules!();

// Commands sent from UI to Logic Thread
enum AppCommand {
    AddSite { domain: String, sitemap: String },
    DeleteSite { id: i64 },
    StartCrawl { site_id: i64, concurrency: usize },
    DeleteCrawl { id: i64 },
    AddQuery { crawl_id: i64, selector: String },
    DeleteQuery { id: i64 },
    LoadResults { query_id: i64 },
    RefreshAll, // To trigger initial load
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Channel for communication: UI -> Logic
    let (tx, mut rx) = mpsc::channel::<AppCommand>(32);

    // Spawn Logic Thread (Single-threaded Tokio Runtime)
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build tokio runtime");

        rt.block_on(async move {
            let app = match Application::new("palimp.db") {
                Ok(app) => Arc::new(app),
                Err(e) => {
                    eprintln!("Failed to initialize application: {}", e);
                    return;
                }
            };

            // Initial load
            refresh_all(&app, &ui_weak).await;

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    AppCommand::AddSite { domain, sitemap } => {
                        if let Err(e) = app.new_site(&domain, &sitemap).await {
                            eprintln!("Error creating site: {}", e);
                        }
                        refresh_sites(&app, &ui_weak).await;
                    }
                    AppCommand::DeleteSite { id } => {
                        if let Err(e) = app.delete_site(id).await {
                            eprintln!("Error deleting site: {}", e);
                        }
                        refresh_sites(&app, &ui_weak).await;
                    }
                    AppCommand::StartCrawl { site_id, concurrency } => {
                        println!("Starting crawl for site {}...", site_id);
                        let result = app.new_crawl(site_id, concurrency, |res| {
                             match res {
                                CrawlResult::PageSucceeded(url) => println!("  [OK] {}", url),
                                CrawlResult::PageFailed(url, err) => eprintln!("  [ERR] {}: {}", url, err),
                            }
                        }).await;
                        
                        if let Err(e) = result {
                            eprintln!("Crawl failed: {}", e);
                        } else {
                            println!("Crawl finished.");
                        }
                        refresh_crawls(&app, &ui_weak).await;
                    }
                    AppCommand::DeleteCrawl { id } => {
                        if let Err(e) = app.delete_crawl(id).await {
                            eprintln!("Error deleting crawl: {}", e);
                        }
                        refresh_crawls(&app, &ui_weak).await;
                    }
                    AppCommand::AddQuery { crawl_id, selector } => {
                         if let Err(e) = app.query(crawl_id, &selector).await {
                             eprintln!("Error running query: {}", e);
                         }
                         refresh_queries(&app, &ui_weak).await;
                    }
                    AppCommand::DeleteQuery { id } => {
                        if let Err(e) = app.delete_query(id).await {
                            eprintln!("Error deleting query: {}", e);
                        }
                        refresh_queries(&app, &ui_weak).await;
                    }
                    AppCommand::LoadResults { query_id } => {
                        refresh_results(&app, &ui_weak, query_id).await;
                    }
                    AppCommand::RefreshAll => {
                        refresh_all(&app, &ui_weak).await;
                    }
                }
            }
        });
    });

    // -- Event Handlers (Main Thread) --
    // These just forward commands to the logic thread

    let tx_clone = tx.clone();
    ui.on_open_add_site_dialog(move || {
        let dialog = AddSiteDialog::new().unwrap();
        let dialog_weak = dialog.as_weak();
        let tx_clone_inner = tx_clone.clone();

        dialog.on_add(move |domain, sitemap| {
             let _ = tx_clone_inner.blocking_send(AppCommand::AddSite { 
                 domain: domain.to_string(), 
                 sitemap: sitemap.to_string() 
             });
             // We can close the dialog here if we want, or let the dialog close itself via button
             // For a separate window, we might want to hide/close it.
             // But Slint windows created this way don't automatically have a close method on the rust side unless we use the window handle.
             // Actually, `dialog` variable is the Window.
             if let Some(d) = dialog_weak.upgrade() {
                 let _ = d.hide();
             }
        });
        
        let dialog_weak = dialog.as_weak();
        dialog.on_cancel_clicked(move || {
            if let Some(d) = dialog_weak.upgrade() {
                let _ = d.hide();
            }
        });

        // Show the dialog as a separate window
        dialog.show().unwrap();
    });

    let tx_clone = tx.clone();
    ui.on_request_add_site(move |domain, sitemap| {
        let _ = tx_clone.blocking_send(AppCommand::AddSite { 
            domain: domain.to_string(), 
            sitemap: sitemap.to_string() 
        });
    });

    let tx_clone = tx.clone();
    ui.on_request_delete_site(move |id_str| {
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::DeleteSite { id });
        }
    });

    let tx_clone = tx.clone();
    ui.on_request_start_crawl(move |site_id_str, concurrency_str| {
        if let Ok(site_id) = site_id_str.parse::<i64>() {
            let concurrency = concurrency_str.parse::<usize>().unwrap_or(5);
            let _ = tx_clone.blocking_send(AppCommand::StartCrawl { site_id, concurrency });
        }
    });

    let tx_clone = tx.clone();
    ui.on_request_delete_crawl(move |id_str| {
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::DeleteCrawl { id });
        }
    });

    let tx_clone = tx.clone();
    ui.on_request_add_query(move |crawl_id_str, selector| {
        if let Ok(crawl_id) = crawl_id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::AddQuery { 
                crawl_id, 
                selector: selector.to_string() 
            });
        }
    });

    let tx_clone = tx.clone();
    ui.on_request_delete_query(move |id_str| {
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::DeleteQuery { id });
        }
    });

    let tx_clone = tx.clone();
    ui.on_request_load_results(move |query_id_str| {
        if let Ok(query_id) = query_id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::LoadResults { query_id });
        }
    });

    ui.run()?;
    Ok(())
}

// -- Helper Functions --

async fn refresh_all(app: &Application, ui_weak: &Weak<AppWindow>) {
    refresh_sites(app, ui_weak).await;
    refresh_crawls(app, ui_weak).await;
    refresh_queries(app, ui_weak).await;
}

// Data structures for passing to UI thread (Send-safe)
struct SiteData { id: String, domain: String, sitemap: String }
struct CrawlData { id: String, site_preview: String, started_at: String }
struct QueryData { id: String, crawl_preview: String, selector: String }
struct ResultData { id: String, page_url: String, count: String }

async fn refresh_sites(app: &Application, ui_weak: &Weak<AppWindow>) {
    let sites = match app.list_sites().await {
        Ok(s) => s,
        Err(e) => { eprintln!("Failed to list sites: {}", e); return; }
    };

    let data: Vec<SiteData> = sites.into_iter().map(|s| SiteData {
        id: s.id.unwrap_or(0).to_string(),
        domain: s.domain,
        sitemap: s.sitemap_url,
    }).collect();

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        let mut items = Vec::new();
        for s in data {
            let row = Rc::new(VecModel::from(vec![
                StandardListViewItem::from(SharedString::from(s.id)),
                StandardListViewItem::from(SharedString::from(s.domain)),
                StandardListViewItem::from(SharedString::from(s.sitemap)),
            ]));
            items.push(ModelRc::from(row));
        }
        ui.set_sites(ModelRc::from(Rc::new(VecModel::from(items))));
    });
}

async fn refresh_crawls(app: &Application, ui_weak: &Weak<AppWindow>) {
    let crawls = match app.list_crawls().await {
        Ok(c) => c,
        Err(e) => { eprintln!("Failed to list crawls: {}", e); return; }
    };

    let sites = app.list_sites().await.unwrap_or_default();
    let site_map: std::collections::HashMap<i64, String> = sites
        .into_iter()
        .filter_map(|s| s.id.map(|id| (id, s.domain)))
        .collect();

    let data: Vec<CrawlData> = crawls.into_iter().map(|c| {
        let site_name = site_map.get(&c.site_id)
            .cloned()
            .unwrap_or_else(|| format!("Unknown Site ({})", c.site_id));
        CrawlData {
            id: c.id.unwrap_or(0).to_string(),
            site_preview: site_name,
            started_at: c.started_at.unwrap_or_default(),
        }
    }).collect();

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        let mut items = Vec::new();
        for c in data {
            let row = Rc::new(VecModel::from(vec![
                StandardListViewItem::from(SharedString::from(c.id)),
                StandardListViewItem::from(SharedString::from(c.site_preview)),
                StandardListViewItem::from(SharedString::from(c.started_at)),
            ]));
            items.push(ModelRc::from(row));
        }
        ui.set_crawls(ModelRc::from(Rc::new(VecModel::from(items))));
    });
}

async fn refresh_queries(app: &Application, ui_weak: &Weak<AppWindow>) {
    let queries = match app.list_queries().await {
        Ok(q) => q,
        Err(e) => { eprintln!("Failed to list queries: {}", e); return; }
    };

    let data: Vec<QueryData> = queries.into_iter().map(|q| QueryData {
        id: q.id.unwrap_or(0).to_string(),
        crawl_preview: format!("Crawl {}", q.crawl_id),
        selector: q.selector,
    }).collect();

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        let mut items = Vec::new();
        for q in data {
            let row = Rc::new(VecModel::from(vec![
                StandardListViewItem::from(SharedString::from(q.id)),
                StandardListViewItem::from(SharedString::from(q.crawl_preview)),
                StandardListViewItem::from(SharedString::from(q.selector)),
            ]));
            items.push(ModelRc::from(row));
        }
        ui.set_queries(ModelRc::from(Rc::new(VecModel::from(items))));
    });
}

async fn refresh_results(app: &Application, ui_weak: &Weak<AppWindow>, query_id: i64) {
    let results = match app.list_results_for_query(query_id).await {
        Ok(r) => r,
        Err(e) => { eprintln!("Failed to load results: {}", e); return; }
    };

    let data: Vec<ResultData> = results.into_iter().map(|(r, url)| ResultData {
        id: r.id.unwrap_or(0).to_string(),
        page_url: url,
        count: r.count.to_string(),
    }).collect();

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        let mut items = Vec::new();
        for r in data {
            let row = Rc::new(VecModel::from(vec![
                StandardListViewItem::from(SharedString::from(r.id)),
                StandardListViewItem::from(SharedString::from(r.page_url)),
                StandardListViewItem::from(SharedString::from(r.count)),
            ]));
            items.push(ModelRc::from(row));
        }
        ui.set_results(ModelRc::from(Rc::new(VecModel::from(items))));
    });
}

