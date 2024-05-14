use axum::{
    routing::get,
    Router,
};

use dotenv::dotenv;
use std::env;


#[tokio::main]
async fn main() {
    dotenv().ok();
    let port = env::var("PORT").unwrap_or("3000".to_string()).parse::<u16>().expect("PORT must be a number");

    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    let listener = tokio::net::TcpListener::bind(format!(":::{}", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
