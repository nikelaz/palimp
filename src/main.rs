use http_client::HTTPClient;

mod http_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new()?;

    let url = "https://www.rust-lang.org";

    let (final_url, response_text) = http_client.get(url).await?;

    println!("Final URL: {}", final_url);

    let dom = tl::parse(response_text.as_str(), tl::ParserOptions::default())
        .map_err(|err| format!("Could not parse response text as HTML for {}:\n{}", url, err))?;

    let selector = "h2";

    let results = dom.query_selector(selector);

    match results {
        Some(res) => println!("Found {} elements for selector {}", res.count(), selector),
        None => println!("No results found"),
    }

    Ok(())
}
