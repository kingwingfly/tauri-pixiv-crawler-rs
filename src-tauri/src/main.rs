// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
use once_cell::sync::OnceCell;
use pixiv_crawler::helper;
use pixiv_crawler::Crawler;

static mut CRAWLER: OnceCell<Crawler> = OnceCell::new();

#[tauri::command]
fn go(uuid: &str, cookie: &str, path: &str, proxy: &str) {
    let rt = helper::create_rt();
    unsafe {
        CRAWLER.take();
        CRAWLER.get_or_init(|| Crawler::new(uuid, cookie, path, proxy));
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![go, interrupt, process])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
