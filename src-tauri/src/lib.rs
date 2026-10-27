pub mod discovery;
pub mod server;

use tauri::{WebviewUrl, WebviewWindowBuilder};

pub fn run() {
    let addr = server::bind_addr();
    // Bind before the window exists so the page never loads against a dead port.
    let listener = std::net::TcpListener::bind(addr).expect("bind web server");
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();

    tauri::Builder::default()
        .setup(move |app| {
            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                server::serve(listener).await;
            });

            let url = if cfg!(debug_assertions) {
                "http://localhost:5173".to_string()
            } else {
                format!("http://127.0.0.1:{port}")
            };
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse().unwrap()))
                .title("Dynacord Config")
                .inner_size(1100.0, 760.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
