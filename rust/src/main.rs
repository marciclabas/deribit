#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

  dotenv::dotenv().ok();
  let client_id = std::env::var("DERIBIT_CLIENT_ID").expect("DERIBIT_CLIENT_ID must be set");
  let client_secret = std::env::var("DERIBIT_CLIENT_SECRET").expect("DERIBIT_CLIENT_SECRET must be set");

  let mut client = deribit::PrivateClient::start_debug(
    deribit::TESTNET,
    &client_id, &client_secret,
    deribit::LogLevel::Debug,
  ).await?;
  let start = std::time::Instant::now();

  // let params = serde_json::json!({
  //   "refresh_token": client.auth.response.refresh_token,
  //   "subject_id": ,
  // });
  // for _ in 0..1 {
  //   let r = client.request("public/exchange_token", params.clone()).await?;
  //   println!("Response: {:?}", r);
  // }

  let r = client.exchange_token(69914).await?;

  let duration = start.elapsed();
  println!("Total time: {:?}", duration);
  Ok(())
}