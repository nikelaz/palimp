use http_client::HTTPClient;
use page::Page;

mod http_client;
mod page;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new()?;

    let url = "https://www.rust-lang.org";

    let (final_url, response_text) = http_client.get(url).await?;

    println!("Final URL: {}", final_url);

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
