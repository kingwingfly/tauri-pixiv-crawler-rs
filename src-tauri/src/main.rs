// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
use once_cell::sync::OnceCell;
use pixiv_crawler::helper;
use pixiv_crawler::Crawler;
use std::collections::HashMap;

static mut CRAWLER: OnceCell<Crawler> = OnceCell::new();

#[tauri::command]
fn go(uuid: &str, cookie: &str, path: &str, proxy: &str) {
    let rt = helper::create_rt();
    let crawler_builder = Crawler::builder()
        .uuid(uuid)
        .cookie(cookie)
        .path(path)
        .proxy(proxy);

    helper::store_builder(&crawler_builder);

    unsafe {
        CRAWLER.take();
        CRAWLER.get_or_init(|| crawler_builder.build());
        rt.block_on(CRAWLER.get().unwrap().run());
    }
}

#[tauri::command]
fn interrupt() {
    let rt = helper::create_rt();
    unsafe {
        rt.block_on(CRAWLER.get().unwrap().shutdown());
    }
}

#[tauri::command]
fn process() -> String {
    unsafe { CRAWLER.get().unwrap().process() }
}

#[tauri::command]
fn download_dir() -> String {
    helper::download_dir()
}

#[tauri::command]
fn save_path() -> String {
    unsafe { CRAWLER.get().unwrap().save_path() }
}

#[tauri::command]
fn get_cached_config() -> HashMap<String, String> {
    helper::get_config()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            go,
            interrupt,
            process,
            download_dir,
            save_path,
            get_cached_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
