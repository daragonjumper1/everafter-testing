use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let directory=app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let database=directory.join("everafter.db");
            // Import the browser prototype only on first launch, using SQLite's online backup.
            // Subsequent launches always use app-owned storage and never overwrite it.
            #[cfg(target_os="windows")]
            if !database.exists() {
                if let Ok(executable)=std::env::current_exe() {
                    if let Some(parent)=executable.parent() {
                        let source=parent.join("everafter").join("everafter.db");
                        if source.exists() { everafter::copy_database_once(&source,&database)?; }
                    }
                }
            }
            let address=tauri::async_runtime::block_on(async move {
                let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await?;
                let address=listener.local_addr()?;
                let router=everafter::router(&database);
                tauri::async_runtime::spawn(async move { if let Err(e)=axum_serve(listener,router).await { eprintln!("Local app server stopped: {e}"); } });
                Ok::<_,std::io::Error>(address)
            })?;
            let window=WebviewWindowBuilder::new(app,"main",WebviewUrl::External(format!("http://{address}/").parse()?))
                .title("Everafter — Your wedding, beautifully together")
                .inner_size(1280.0,850.0).min_inner_size(360.0,600.0)
                .build()?;
            #[cfg(desktop)]
            {let _=window.set_focus();}
            #[cfg(mobile)]
            {let _=window;}
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Could not start Everafter");
}

async fn axum_serve(listener:tokio::net::TcpListener,router:everafter::AppRouter)->std::io::Result<()> {
    everafter::serve(listener,router).await
}
