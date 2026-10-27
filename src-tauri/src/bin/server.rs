//! Headless mode: just the web server, no window. Open http://<host>:7070 in any browser.

#[tokio::main]
async fn main() {
    let addr = dynacord_config_lib::server::bind_addr();
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    println!("Dynacord Config at http://{}", listener.local_addr().unwrap());
    dynacord_config_lib::server::serve(listener).await;
}
