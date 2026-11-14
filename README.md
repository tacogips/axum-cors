# This repository is DEPRECATED
This repository was worked on axum 0.2. `axum >=0.4` adopted another cors implementation of `tower-http>=2.0`

See [The example](https://github.com/tokio-rs/axum/blob/v0.4.0/examples/cors/src/main.rs)

## A CORS layer for [tokio-rs/axum](https://github.com/tokio-rs/axum).

The logic it self is copied from [tower-web](https://github.com/carllerche/tower-web)

## install
```
# Cargo.toml
axum-cors = { git = "https://github.com/tacogips/axum-cors.git", branch="master" }
```


## usage
```rust
use axum::{handler::get, Router};
use axum_cors::*;
use http::{header, Method};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let cors_layer = CorsBuilder::new()
        .allow_origins(AllowedOrigins::Any { allow_null: false })
        .allow_headers(&[
            header::ACCEPT,
            header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            header::ACCEPT_ENCODING,
            header::ACCEPT_LANGUAGE,
            header::AUTHORIZATION,
        ])
        .allow_methods(&[Method::GET])
        .into_layer();

    let app = Router::new().route("/", get(handler)).layer(cors_layer);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> &'static str {
    "<h1>CORS check passed </h1>"
}
```
