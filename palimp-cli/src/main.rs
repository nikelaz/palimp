use palimp_core::{Application, CrawlResult};
use std::env;
use std::error::Error;
use std::process;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let app = Application::new("palimp.db")?;
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    match args[1].as_str() {
        "sites" => handle_sites(&app, &args[2..]).await?,
        "crawls" => handle_crawls(&app, &args[2..]).await?,
        "queries" => handle_queries(&app, &args[2..]).await?,
        "results" => handle_results(&app, &args[2..]).await?,
        "export" => handle_export(&app, &args[2..]).await?,
        _ => print_help(),
    }

    Ok(())
}

async fn handle_sites(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let sites = app.list_sites().await?;
            if sites.is_empty() {
                println!("No sites found.");
            } else {
                println!("{:<5} {:<30} {:<50}", "ID", "Domain", "Sitemap URL");
                println!("{:-<5} {:-<30} {:-<50}", "", "", "");
                for site in sites {
                    println!(
                        "{:<5} {:<30} {:<50}",
                        site.id.unwrap_or(0),
                        site.domain,
                        site.sitemap_url
                    );
                }
            }
        }
        "new" => {
            if args.len() != 3 {
                println!("Usage: sites new <domain> <sitemap_url>");
                return Ok(());
            }
            app.new_site(&args[1], &args[2]).await?;
            println!("Site created successfully.");
        }
        "delete" => {
            if args.len() != 2 {
                println!("Usage: sites delete <id>");
                return Ok(());
            }
            let id = args[1].parse::<i64>()?;
            app.delete_site(id).await?;
            println!("Site deleted successfully.");
        }
        _ => print_help(),
    }
    Ok(())
}

async fn handle_crawls(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let crawls = app.list_crawls().await?;
            let sites = app.list_sites().await?;

            let site_map: std::collections::HashMap<i64, String> = sites
                .into_iter()
                .filter_map(|s| s.id.map(|id| (id, s.domain)))
                .collect();

            if crawls.is_empty() {
                println!("No crawls found.");
            } else {
                println!("{:<5} {:<40} {:<30}", "ID", "Site", "Started At");
                println!("{:-<5} {:-<40} {:-<30}", "", "", "");
                for crawl in crawls {
                    let site_display = match site_map.get(&crawl.site_id) {
                        Some(domain) => format!("{} (ID: {})", domain, crawl.site_id),
                        None => format!("Unknown (ID: {})", crawl.site_id),
                    };

                    println!(
                        "{:<5} {:<40} {:<30}",
                        crawl.id.unwrap_or(0),
                        site_display,
                        crawl.started_at.as_deref().unwrap_or("Unknown")
                    );
                }
            }
        }
        "new" => {
            if args.len() < 2 {
                println!("Usage: crawls new <site_id> [max_concurrent]");
                return Ok(());
            }
            let site_id = args[1].parse::<i64>()?;
            let max_concurrent = if args.len() >= 3 {
                args[2].parse::<usize>().unwrap_or(5)
            } else {
                5
            };

            println!("Starting crawl for site {} with concurrency {}...", site_id, max_concurrent);
            
            app.new_crawl(site_id, max_concurrent, |result| {
                match result {
                    CrawlResult::CrawlStarted(total) => {
                        println!("Crawling {} pages...", total);
                    }
                    CrawlResult::PageSucceeded(url) => println!("  [OK] {}", url),
                    CrawlResult::PageFailed(url, err) => eprintln!("  [ERR] {}: {}", url, err),
                }
            }).await?;
            
            println!("Crawl completed.");
        }
        "delete" => {
            if args.len() != 2 {
                println!("Usage: crawls delete <id>");
                return Ok(());
            }
            let id = args[1].parse::<i64>()?;
            app.delete_crawl(id).await?;
            println!("Crawl deleted successfully.");
        }
        _ => print_help(),
    }
    Ok(())
}

async fn handle_queries(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let queries = app.list_queries().await?;
            let crawls = app.list_crawls().await?;
            let sites = app.list_sites().await?;

            let site_map: std::collections::HashMap<i64, String> = sites
                .into_iter()
                .filter_map(|s| s.id.map(|id| (id, s.domain)))
                .collect();

            let crawl_map: std::collections::HashMap<i64, (i64, Option<String>)> = crawls
                .into_iter()
                .filter_map(|c| c.id.map(|id| (id, (c.site_id, c.started_at))))
                .collect();

            if queries.is_empty() {
                println!("No queries found.");
            } else {
                println!("{:<5} {:<60} {:<30}", "ID", "Crawl", "Selector");
                println!("{:-<5} {:-<60} {:-<30}", "", "", "");
                for query in queries {
                    let crawl_display = match crawl_map.get(&query.crawl_id) {
                        Some((site_id, started_at)) => {
                            let domain = site_map.get(site_id).map(|s| s.as_str()).unwrap_or("Unknown Site");
                            let timestamp = started_at.as_deref().unwrap_or("Unknown Time");
                            format!("{} (Crawl ID: {}) {}", domain, query.crawl_id, timestamp)
                        }
                        None => format!("Unknown Crawl (ID: {})", query.crawl_id),
                    };

                    println!(
                        "{:<5} {:<60} {:<30}",
                        query.id.unwrap_or(0),
                        crawl_display,
                        query.selector
                    );
                }
            }
        }
        "new" => {
            if args.len() != 3 {
                println!("Usage: queries new <crawl_id> <selector>");
                return Ok(());
            }
            let crawl_id = args[1].parse::<i64>()?;
            let selector = &args[2];
            
            println!("Running query '{}' on crawl {}...", selector, crawl_id);
            let results = app.query(crawl_id, selector).await?;
            println!("Query completed. Found {} matching results across pages.", results.len());
        }
        "delete" => {
            if args.len() != 2 {
                println!("Usage: queries delete <id>");
                return Ok(());
            }
            let id = args[1].parse::<i64>()?;
            app.delete_query(id).await?;
            println!("Query deleted successfully.");
        }
        _ => print_help(),
    }
    Ok(())
}

async fn handle_results(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    // We expect the first argument to be the query_id
    let query_id = match args[0].parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid Query ID. Please provide a numeric ID.");
            print_help();
            return Ok(());
        }
    };

    let results = app.list_results_for_query(query_id).await?;

    if results.is_empty() {
        println!("No results found for query ID {}.", query_id);
    } else {
        println!("{:<5} {:<60} {:<10}", "ID", "Page URL", "Count");
        println!("{:-<5} {:-<60} {:-<10}", "", "", "");
        for (res, url) in results {
            println!(
                "{:<5} {:<60} {:<10}",
                res.id.unwrap_or(0),
                url,
                res.count
            );
        }
    }

    Ok(())
}

async fn handle_export(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 2 {
        println!("Usage: export <query_id> <csv_filename>");
        return Ok(());
    }

    let query_id = match args[0].parse::<i64>() {
        Ok(id) => id,
        Err(_) => {
            println!("Invalid Query ID. Please provide a numeric ID.");
            return Ok(());
        }
    };
    
    let filename = &args[1];
    
    let results = app.list_results_for_query(query_id).await?;

    if results.is_empty() {
        println!("No results found for query ID {}. Nothing to export.", query_id);
        return Ok(());
    }

    let mut wtr = csv::Writer::from_path(filename)?;
    
    // Write header
    wtr.write_record(&["ID", "Page URL", "Count"])?;

    for (res, url) in results {
        wtr.write_record(&[
            res.id.unwrap_or(0).to_string(),
            url,
            res.count.to_string(),
        ])?;
    }
    
    wtr.flush()?;
    println!("Successfully exported results to '{}'.", filename);

    Ok(())
}

fn print_help() {
    println!("Usage: palimp-cli <command> [subcommand] [args]");
    println!("\nCommands:");
    println!("  sites list");
    println!("  sites new <domain> <sitemap_url>");
    println!("  sites delete <id>");
    println!();
    println!("  crawls list");
    println!("  crawls new <site_id> [max_concurrent]");
    println!("  crawls delete <id>");
    println!();
    println!("  queries list");
    println!("  queries new <crawl_id> <selector>");
    println!("  queries delete <id>");
    println!();
    println!("  results <query_id>");
    println!();
    println!("  export <query_id> <csv_filename>");
}
