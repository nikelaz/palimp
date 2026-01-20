use http_client::HTTPClient;

mod http_client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let http_client = HTTPClient::new()?;

    let url = "https://www.rust-lang.org";

    let (final_url, response_text) = http_client.get(url).await?;

    println!("{}", response_text);
    println!("Final URL: {}", final_url);

    Ok(())
}
