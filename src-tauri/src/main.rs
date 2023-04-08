// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
use once_cell::sync::OnceCell;
use pixiv_crawler::helper;
use pixiv_crawler::{Crawler, TaskMng};

static CRAWLER: OnceCell<Crawler> = OnceCell::new();
static TASKMNG: OnceCell<TaskMng> = OnceCell::new();

#[tauri::command]
fn go(uuid: &str, cookie: &str, path: &str, proxy: &str) {}

#[tauri::command]
fn interrupt() {
    CRAWLER.get_or_init(|| Crawler::new());
    TASKMNG.get_or_init(|| TaskMng::new());
    TASKMNG
        .get()
        .unwrap()
        .spawn_task(CRAWLER.get().unwrap().run(3));
}

#[tauri::command]
fn process() -> String {
    TASKMNG.get().unwrap().process()
}

fn main() {
    interrupt();
    // tauri::Builder::default()
    //     .invoke_handler(tauri::generate_handler![go, interrupt, process])
    //     .run(tauri::generate_context!())
    //     .expect("error while running tauri application");
}
