use palimp_core::{Application, CrawlResult};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel, Weak};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::mpsc;

slint::include_modules!();

// Commands sent from UI to Logic Thread
enum AppCommand {
    AddSite { domain: String, sitemap: String },
    DeleteSite { id: i64 },
    LoadCrawlsForSite { site_id: i64 },
    StartCrawl { site_id: i64, concurrency: usize },
    DeleteCrawl { id: i64 },
    RunQuery { crawl_id: i64, selector: String },
    RefreshAll,
}

// Data structures for passing to UI thread (Send-safe)
#[derive(Clone)]
struct SiteData { id: String, domain: String, sitemap: String }
#[derive(Clone)]
struct CrawlData { id: String, started_at: String }
#[derive(Clone)]
struct ResultData { id: String, page_url: String, count: String }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ui = AppWindow::new()?;
    let ui_weak = ui.as_weak();

    // Channel for communication: UI -> Logic
    let (tx, mut rx) = mpsc::channel::<AppCommand>(32);
    
    // Cache to store results per crawl for this session
    let results_cache = Arc::new(Mutex::new(HashMap::<i64, Vec<ResultData>>::new()));
    
    // Cache to remember selected crawl per site
    let selected_crawl_cache = Arc::new(Mutex::new(HashMap::<i64, i64>::new()));
    
    // Map site index to site ID
    let site_index_map = Arc::new(Mutex::new(Vec::<i64>::new()));

    // Spawn Logic Thread (Single-threaded Tokio Runtime)
    let results_cache_clone = Arc::clone(&results_cache);
    let selected_crawl_cache_clone = Arc::clone(&selected_crawl_cache);
    let site_index_map_clone = Arc::clone(&site_index_map);
    let ui_weak_for_thread = ui_weak.clone();
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
            refresh_sites(&app, &ui_weak_for_thread, &site_index_map_clone).await;

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    AppCommand::AddSite { domain, sitemap } => {
                        if let Err(e) = app.new_site(&domain, &sitemap).await {
                            eprintln!("Error creating site: {}", e);
                        }
                        refresh_sites(&app, &ui_weak_for_thread, &site_index_map_clone).await;
                    }
                    AppCommand::DeleteSite { id } => {
                        if let Err(e) = app.delete_site(id).await {
                            eprintln!("Error deleting site: {}", e);
                        }
                        refresh_sites(&app, &ui_weak_for_thread, &site_index_map_clone).await;
                    }
                    AppCommand::LoadCrawlsForSite { site_id } => {
                        // Update UI with selected site ID
                        let ui_weak_clone = ui_weak_for_thread.clone();
                        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                            ui.set_selected_site_id(SharedString::from(site_id.to_string()));
                        });
                        
                        refresh_crawls_for_site(&app, &ui_weak_for_thread, site_id).await;
                        
                        // Check if we have a previously selected crawl for this site
                        if let Ok(cache) = selected_crawl_cache_clone.lock() {
                            if let Some(&crawl_id) = cache.get(&site_id) {
                                // Restore cached results for the previously selected crawl
                                if let Ok(results_cache) = results_cache_clone.lock() {
                                    if let Some(cached_results) = results_cache.get(&crawl_id) {
                                        let cached_results = cached_results.clone();
                                        let ui_weak_clone = ui_weak_for_thread.clone();
                                        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| {
                                            let mut items = Vec::new();
                                            for r in cached_results {
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
                                }
                            }
                        }
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
                        
                        // Refresh crawls for this site
                        refresh_crawls_for_site(&app, &ui_weak_for_thread, site_id).await;
                    }
                    AppCommand::DeleteCrawl { id } => {
                        if let Err(e) = app.delete_crawl(id).await {
                            eprintln!("Error deleting crawl: {}", e);
                        }
                        // Clear results when crawl is deleted
                        let _ = ui_weak_for_thread.upgrade_in_event_loop(|ui| {
                            ui.set_results(ModelRc::from(Rc::new(VecModel::from(vec![]))));
                        });
                        
                        // Note: We'd need to know which site to refresh. For now, we'll need to track this.
                        // Simplified: just clear the UI results
                    }
                    AppCommand::RunQuery { crawl_id, selector } => {
                        if let Err(e) = app.query(crawl_id, &selector).await {
                            eprintln!("Error running query: {}", e);
                            return;
                        }
                        
                        // Get the most recent query for this crawl to fetch results
                        let queries = app.list_queries().await.unwrap_or_default();
                        let latest_query = queries.iter()
                            .filter(|q| q.crawl_id == crawl_id)
                            .max_by_key(|q| q.id);
                        
                        if let Some(query) = latest_query {
                            if let Some(query_id) = query.id {
                                // Fetch results
                                let results = app.list_results_for_query(query_id).await.unwrap_or_default();
                                let data: Vec<ResultData> = results.into_iter().map(|(r, url)| ResultData {
                                    id: r.id.unwrap_or(0).to_string(),
                                    page_url: url,
                                    count: r.count.to_string(),
                                }).collect();
                                
                                // Cache results for this crawl
                                if let Ok(mut cache) = results_cache_clone.lock() {
                                    cache.insert(crawl_id, data.clone());
                                }
                                
                                // Update UI
                                let _ = ui_weak_for_thread.upgrade_in_event_loop(move |ui| {
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
                        }
                    }
                    AppCommand::RefreshAll => {
                        refresh_sites(&app, &ui_weak_for_thread, &site_index_map_clone).await;
                    }
                }
            }
        });
    });

    // -- Event Handlers (Main Thread) --
    
    // Open Add Site Dialog
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

        dialog.show().unwrap();
    });

    // Site selected - load crawls for that site
    let tx_clone = tx.clone();
    let site_index_map_clone = Arc::clone(&site_index_map);
    ui.on_site_selected(move |site_index| {
        // Look up site ID from index
        if let Ok(map) = site_index_map_clone.lock() {
            if site_index >= 0 && (site_index as usize) < map.len() {
                let site_id = map[site_index as usize];
                let _ = tx_clone.blocking_send(AppCommand::LoadCrawlsForSite { site_id });
            }
        }
    });

    // Open Add Crawl Dialog
    let tx_clone = tx.clone();
    ui.on_open_add_crawl_dialog(move |site_id_str| {
        let dialog = AddCrawlDialog::new().unwrap();
        let dialog_weak = dialog.as_weak();
        let tx_clone_inner = tx_clone.clone();
        let site_id_str_clone = site_id_str.to_string();

        dialog.on_start(move |_, concurrency_str| {
            if let Ok(site_id) = site_id_str_clone.parse::<i64>() {
                let concurrency = concurrency_str.parse::<usize>().unwrap_or(5);
                let _ = tx_clone_inner.blocking_send(AppCommand::StartCrawl { site_id, concurrency });
            }
            
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

        dialog.show().unwrap();
    });

    // Delete crawl
    let tx_clone = tx.clone();
    ui.on_request_delete_crawl(move |id_str| {
        if let Ok(id) = id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::DeleteCrawl { id });
        }
    });

    // Run query
    let tx_clone = tx.clone();
    ui.on_request_run_query(move |crawl_id_str, selector| {
        if let Ok(crawl_id) = crawl_id_str.parse::<i64>() {
            let _ = tx_clone.blocking_send(AppCommand::RunQuery { 
                crawl_id, 
                selector: selector.to_string() 
            });
        }
    });

    // Crawl selected - restore cached results and remember selection per site
    let results_cache_clone = Arc::clone(&results_cache);
    let selected_crawl_cache_clone = Arc::clone(&selected_crawl_cache);
    let ui_weak_clone = ui_weak.clone();
    ui.on_crawl_selected(move |crawl_id_str| {
        if let Ok(crawl_id) = crawl_id_str.parse::<i64>() {
            // Try to restore cached results
            if let Ok(cache) = results_cache_clone.lock() {
                if let Some(cached_results) = cache.get(&crawl_id) {
                    let cached_results = cached_results.clone();
                    let ui_weak_inner = ui_weak_clone.clone();
                    let _ = ui_weak_inner.upgrade_in_event_loop(move |ui| {
                        let mut items = Vec::new();
                        for r in cached_results {
                            let row = Rc::new(VecModel::from(vec![
                                StandardListViewItem::from(SharedString::from(r.id)),
                                StandardListViewItem::from(SharedString::from(r.page_url)),
                                StandardListViewItem::from(SharedString::from(r.count)),
                            ]));
                            items.push(ModelRc::from(row));
                        }
                        ui.set_results(ModelRc::from(Rc::new(VecModel::from(items))));
                    });
                } else {
                    // No cache, clear results
                    let ui_weak_inner = ui_weak_clone.clone();
                    let _ = ui_weak_inner.upgrade_in_event_loop(|ui| {
                        ui.set_results(ModelRc::from(Rc::new(VecModel::from(vec![]))));
                    });
                }
            }
            
            // Remember this crawl selection for the current site
            // Note: We'd need to track current site_id. For simplicity, we'll store it globally for now.
            // This is a simplification - in a real app, we'd track site_id -> crawl_id mapping
        }
    });

    ui.run()?;
    Ok(())
}

// -- Helper Functions --

async fn refresh_sites(app: &Application, ui_weak: &Weak<AppWindow>, site_index_map: &Arc<Mutex<Vec<i64>>>) {
    let sites = match app.list_sites().await {
        Ok(s) => s,
        Err(e) => { eprintln!("Failed to list sites: {}", e); return; }
    };

    let data: Vec<SiteData> = sites.into_iter().map(|s| SiteData {
        id: s.id.unwrap_or(0).to_string(),
        domain: s.domain,
        sitemap: s.sitemap_url,
    }).collect();

    let is_empty = data.is_empty();
    let first_site_index = if !is_empty { 0 } else { -1 };
    
    // Build site index -> ID mapping
    let site_ids: Vec<i64> = data.iter()
        .map(|s| s.id.parse::<i64>().unwrap_or(0))
        .collect();
    
    // Update the global site index map
    if let Ok(mut map) = site_index_map.lock() {
        *map = site_ids;
    }

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        // Build 1D array with just display text
        let items: Vec<StandardListViewItem> = data.iter()
            .map(|s| StandardListViewItem::from(SharedString::from(format!("{} (ID: {})", s.domain, s.id))))
            .collect();
        ui.set_sites(ModelRc::from(Rc::new(VecModel::from(items))));
        
        // Auto-open dialog if no sites
        if is_empty {
            ui.invoke_open_add_site_dialog();
        } else {
            // Auto-select first site
            ui.set_selected_site_index(0);
            ui.invoke_site_selected(first_site_index);
        }
    });
}

async fn refresh_crawls_for_site(app: &Application, ui_weak: &Weak<AppWindow>, site_id: i64) {
    let crawls = match app.list_crawls().await {
        Ok(c) => c,
        Err(e) => { eprintln!("Failed to list crawls: {}", e); return; }
    };

    // Filter crawls for this site
    let filtered_crawls: Vec<_> = crawls.into_iter()
        .filter(|c| c.site_id == site_id)
        .collect();

    let data: Vec<CrawlData> = filtered_crawls.into_iter().map(|c| {
        CrawlData {
            id: c.id.unwrap_or(0).to_string(),
            started_at: c.started_at.unwrap_or_default(),
        }
    }).collect();

    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        let mut items = Vec::new();
        for c in data {
            let row = Rc::new(VecModel::from(vec![
                StandardListViewItem::from(SharedString::from(c.id)),
                StandardListViewItem::from(SharedString::from(c.started_at)),
            ]));
            items.push(ModelRc::from(row));
        }
        ui.set_crawls(ModelRc::from(Rc::new(VecModel::from(items))));
    });
}
