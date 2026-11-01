#[tokio::main]
async fn main() {
    let path=std::env::var("EVERAFTER_DB_PATH").unwrap_or_else(|_|"everafter.db".into());
    let app=everafter::router(std::path::Path::new(&path));
    let address=std::env::var("EVERAFTER_BIND").unwrap_or_else(|_|"127.0.0.1:3000".into());
    let listener=tokio::net::TcpListener::bind(&address).await.expect("Bind server");
    println!("Everafter is ready at http://{address}");
    everafter::serve(listener,app).await.expect("Serve app");
}
