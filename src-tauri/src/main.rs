// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
use pixiv_crawler::{Crawler, CrawlerTrait};

#[tauri::command]
fn go(uuid: &str, cookie: &str, path: &str, proxy: &str) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let crawler = Crawler::new(uuid, cookie, proxy, path);
    rt.block_on(crawler.run());
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![go])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
