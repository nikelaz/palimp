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
        "query" => handle_query(&app, &args[2..]).await?,
        "results" => handle_results(&app, &args[2..]).await?,
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
            if crawls.is_empty() {
                println!("No crawls found.");
            } else {
                println!("{:<5} {:<10} {:<30}", "ID", "Site ID", "Started At");
                println!("{:-<5} {:-<10} {:-<30}", "", "", "");
                for crawl in crawls {
                    println!(
                        "{:<5} {:<10} {:<30}",
                        crawl.id.unwrap_or(0),
                        crawl.site_id,
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

async fn handle_query(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.len() != 2 {
        println!("Usage: query <crawl_id> <selector>");
        return Ok(());
    }

    let crawl_id = args[0].parse::<i64>()?;
    let selector = &args[1];

    println!("Running query '{}' on crawl {}...", selector, crawl_id);
    let results = app.query(crawl_id, selector).await?;
    println!("Query completed. Found {} matching results across pages.", results.len());

    Ok(())
}

async fn handle_results(app: &Application, args: &[String]) -> Result<(), Box<dyn Error>> {
    if args.is_empty() {
        print_help();
        return Ok(());
    }

    match args[0].as_str() {
        "list" => {
            let results = app.list_results().await?;
            if results.is_empty() {
                println!("No results found.");
            } else {
                println!("{:<5} {:<10} {:<30} {:<10}", "ID", "Page ID", "Selector", "Count");
                println!("{:-<5} {:-<10} {:-<30} {:-<10}", "", "", "", "");
                for res in results {
                    println!(
                        "{:<5} {:<10} {:<30} {:<10}",
                        res.id.unwrap_or(0),
                        res.page_id,
                        res.selector,
                        res.count
                    );
                }
            }
        }
        "delete" => {
            if args.len() != 2 {
                println!("Usage: results delete <id>");
                return Ok(());
            }
            let id = args[1].parse::<i64>()?;
            app.delete_result(id).await?;
            println!("Result deleted successfully.");
        }
        _ => print_help(),
    }
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
    println!("  query <crawl_id> <selector>");
    println!();
    println!("  results list");
    println!("  results delete <id>");
}
