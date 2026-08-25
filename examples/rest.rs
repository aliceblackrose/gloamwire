use gloamwire::RestClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("DISCORD_TOKEN")?;
    let client = RestClient::new(token)?;
    let user = client.get_current_user().await?;

    println!("authenticated as {} ({})", user.username, user.id);
    Ok(())
}
