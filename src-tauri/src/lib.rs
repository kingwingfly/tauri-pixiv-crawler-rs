use futures::Future;
use reqwest::{
    header::{self, HeaderMap},
    Proxy,
};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub struct Crawler {
    uuid: String,
    cookie: String,
    path: String,
    proxy: Option<Proxy>,
    tk_mng: TaskMng,
}

impl Crawler {
    pub fn new(uuid: &str, cookie: &str, path: &str, proxy: &str) -> Self {
        let proxy = Proxy::all(proxy.trim()).ok();
        let path = match path.trim() {
            "" => helper::download_dir(),
            _ => path.trim().to_owned(),
        };
        let path = format!("{}/{}", path, uuid.trim());
        let tk_mng = TaskMng::new();
        Self {
            uuid: uuid.trim().into(),
            cookie: cookie.trim().into(),
            path,
            proxy,
            tk_mng,
        }
    }

    pub fn builder() -> CrawlerBuilder {
        CrawlerBuilder::new()
    }

    pub async fn run(&self) {
        fs::create_dir_all(&self.path).await.unwrap();
        let illu_ids = loop {
            if let Ok(illu_ids) = self.step1().await {
                break illu_ids;
            }
        };
        for illu_id in illu_ids {
            let illu_ajax = self.illu_ajax(&illu_id);
            let proxy = self.proxy.clone();
            let headers = self.headers(&illu_id);
            let path = self.path.clone();
            self.tk_mng
                .spawn_task(async move {
                    let ori_urls = loop {
                        if let Ok(illu_ids) = Self::step2(illu_ajax.clone(), proxy.clone()).await {
                            break illu_ids;
                        }
                    };
                    for ori_url in ori_urls {
                        Self::step3(ori_url, headers.clone(), proxy.clone(), path.clone()).await;
                    }
                })
                .await;
        }
    }

    async fn step1(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let url = self.user_ajax();
        let client = helper::create_client(self.proxy.clone());
        let resp = client.get(&url).send().await?;
        let content = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        let illus = v
            .get("body")
            .and_then(|v| v.get("illusts"))
            .map(|v| {
                v.as_object()
                    .unwrap()
                    .keys()
                    .map(|k| k.to_string())
                    .collect()
            })
            .unwrap_or(Vec::new());

        // v
        //     .get("body")
        //     .unwrap()
        //     .get("illusts")
        //     .unwrap()
        //     .as_object()
        //     .unwrap()
        //     .keys()
        //     .map(|k| k.to_string())
        //     .collect();
        Ok(illus)
    }

    async fn step2(
        illu_ajax: String,
        proxy: Option<Proxy>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let client = helper::create_client(proxy);
        let resp = client.get(illu_ajax).send().await?;
        let content = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        let ori_urls: Vec<String> = v
            .get("body")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                i.get("urls")
                    .unwrap()
                    .get("original")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        Ok(ori_urls)
    }

    async fn step3(ori_url: String, headers: HeaderMap, proxy: Option<Proxy>, path: String) {
        let name = ori_url.split('/').last().unwrap();
        let path = format!("{}/{}", path, name);
        if fs::try_exists(&path).await.unwrap() {
            return;
        }
        println!("Doing: {}", ori_url);
        loop {
            let client = helper::create_client(proxy.clone());
            let resp = match client.get(&ori_url).headers(headers.clone()).send().await {
                Ok(resp) => resp,
                Err(_) => continue,
            };
            let content = match resp.bytes().await {
                Ok(content) => content,
                Err(_) => continue,
            };
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .open(path.clone())
                .await
                .unwrap();
            file.write_all(&content).await.unwrap();
            break;
        }
        println!("Done: {}", ori_url);
    }

    pub async fn shutdown(&self) {
        self.tk_mng.shutdown().await;
    }

    pub fn process(&self) -> String {
        self.tk_mng.process()
    }

    pub fn save_path(&self) -> String {
        self.path.clone()
    }

    fn user_ajax(&self) -> String {
        format!(
            "https://www.pixiv.net/ajax/user/{}/profile/all?lang=zh",
            &self.uuid[..]
        )
    }

    fn illu_ajax(&self, illu_id: &str) -> String {
        format!("https://www.pixiv.net/ajax/illust/{illu_id}/pages?lang=zh")
    }

    fn headers(&self, illu_id: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, self.cookie.parse().unwrap());
        headers.append(
            header::REFERER,
            format!("https://www.pixiv.net/artworks/{illu_id}")
                .parse()
                .unwrap(),
        );
        headers.append(header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Safari/605.1.15".parse().unwrap());
        headers
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlerBuilder {
    uuid: Option<String>,
    cookie: Option<String>,
    path: Option<String>,
    proxy: Option<String>,
}

impl CrawlerBuilder {
    pub fn new() -> Self {
        Self {
            uuid: None,
            cookie: None,
            path: None,
            proxy: None,
        }
    }

    pub fn build(self) -> Crawler {
        let uuid = self.uuid.unwrap();
        let cookie = self.cookie.unwrap();
        let path = self.path.unwrap_or("".to_owned());
        let proxy = self.proxy.unwrap_or("".to_owned());
        Crawler::new(&uuid, &cookie, &path, &proxy)
    }

    pub fn uuid(mut self, uuid: &str) -> Self {
        self.uuid = Some(uuid.trim().to_owned());
        self
    }

    pub fn cookie(mut self, cookie: &str) -> Self {
        self.cookie = Some(cookie.trim().to_owned());
        self
    }

    pub fn path(mut self, path: &str) -> Self {
        let path = match path.trim() {
            "" => helper::download_dir(),
            _ => path.trim().to_owned(),
        };
        self.path = Some(path.trim().to_owned());
        self
    }

    pub fn proxy(mut self, proxy: &str) -> Self {
        self.proxy = Some(proxy.trim().to_owned());
        self
    }
}

struct TaskMng {
    tx: mpsc::Sender<Message>,
    total: AtomicUsize,
    finished: Arc<AtomicUsize>,
}

struct Task {
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
}

enum Message {
    Job(Task),
    Terminate,
}

impl TaskMng {
    fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<Message>(16);
        let finished = Arc::new(AtomicUsize::new(0));
        let finished_clone = finished.clone();
        std::thread::spawn(move || {
            let rt = helper::create_rt();
            rt.block_on(async move {
                while let Some(msg) = rx.recv().await {
                    match msg {
                        Message::Job(task) => {
                            let finished_clone_clone = finished_clone.clone();
                            tokio::spawn(async move {
                                task.task.await;
                                finished_clone_clone.fetch_add(1, Ordering::SeqCst);
                            });
                        }
                        Message::Terminate => break,
                    }
                }
                println!("Shut down")
            })
        });

        Self {
            tx,
            total: AtomicUsize::new(0),
            finished,
        }
    }

    async fn spawn_task<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
        F::Output: Send,
    {
        let task = Task {
            task: Box::pin(task),
        };
        let msg = Message::Job(task);
        match self.tx.send(msg).await {
            Ok(()) => {
                self.total.fetch_add(1, Ordering::SeqCst);
            }
            Err(_) => panic!("The shared runtime has shut down."),
        };
    }

    fn process(&self) -> String {
        format!(
            "{}/{}",
            self.finished.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed)
        )
    }

    pub async fn shutdown(&self) {
        self.tx.send(Message::Terminate).await.ok().unwrap();
    }
}

pub mod helper {
    use super::CrawlerBuilder;
    use keyring::Entry;
    use reqwest::{Client, Proxy};
    use std::collections::HashMap;
    use std::env;
    use std::fs;
    use tauri::api::path;

    pub fn create_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    }

    pub fn create_client(proxy: Option<Proxy>) -> Client {
        match proxy {
            Some(proxy) => Client::builder().proxy(proxy).build().unwrap(),
            None => Client::new(),
        }
    }

    pub fn download_dir() -> String {
        path::download_dir().unwrap().to_str().unwrap().to_owned()
    }

    pub fn config_dir() -> std::path::PathBuf {
        let mut path = path::config_dir().unwrap();
        path.push("PixivCrawler");
        fs::create_dir_all(&path).unwrap();
        path
    }

    pub fn store_builder(builder: &CrawlerBuilder) {
        let mut path = config_dir();
        path.push("config.json");
        let json = serde_json::to_string(&builder).unwrap();
        let user = match env::var("USERNAME") {
            Ok(user) => user,
            Err(_) => env::var("USER").unwrap(),
        };
        let entry = Entry::new("pixiv_downloader", &user).unwrap();
        entry.set_password(&json).unwrap();

        // let mut file = OpenOptions::new()
        //     .create(true)
        //     .write(true)
        //     .truncate(true)
        //     .open(path)
        //     .unwrap();
        // file.write(json.as_bytes()).unwrap();
    }

    pub fn get_config() -> HashMap<String, String> {
        let user = match env::var("USERNAME") {
            Ok(user) => user,
            Err(_) => env::var("USER").unwrap(),
        };
        let entry = Entry::new("pixiv_downloader", &user).unwrap();
        match entry.get_password() {
            Ok(config) => serde_json::from_str(&config).unwrap(),
            Err(_) => {
                let json = r#"
                {
                    "uuid": "",
                    "cookie": "",
                    "path": "",
                    "proxy": ""
                }
                "#;
                serde_json::from_str(json).unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskmng_test() {
        let rt = helper::create_rt();
        rt.block_on(async {
            let taskmng = TaskMng::new();
            taskmng.spawn_task(async {}).await;
            std::thread::sleep(std::time::Duration::from_secs(1));
            assert_eq!(taskmng.process(), "1/1");
            taskmng.spawn_task(async {}).await;
            std::thread::sleep(std::time::Duration::from_secs(1));
            assert_eq!(taskmng.process(), "2/2");
        });
    }

    #[test]
    #[should_panic(expected = "The shared runtime has shut down.")]
    fn shut_down_test() {
        let rt = helper::create_rt();
        rt.block_on(async {
            let taskmng = TaskMng::new();
            taskmng.shutdown().await;
            std::thread::sleep(std::time::Duration::from_secs(1));
            taskmng.spawn_task(async {}).await;
        });
    }

    #[test]
    fn crawler_test() {
        let rt = helper::create_rt();
        let uuid = "1960050";
        let cookie = "PHPSESSID=33192056_jFm1Tvb7uPEaBHSAOlMw7JZauSY6bO8a";
        let path = "./Downloads";
        let proxy = "http://127.0.0.1:7890";
        let crawler = Crawler::new(uuid, cookie, path, proxy);
        rt.block_on(async {
            crawler.run().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            crawler.shutdown().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
            println!("{}", crawler.process());
        });
    }

    #[test]
    fn build_test() {
        let builder = Crawler::builder().uuid("1").cookie("2").path("").proxy("");
        let crawler = builder.build();
        assert_eq!(crawler.uuid, "1");
        assert_eq!(crawler.cookie, "2");
        assert_eq!(crawler.path, helper::download_dir() + "/1");
        assert!(crawler.proxy.is_none());
        let builder = Crawler::builder()
            .uuid("1")
            .cookie("2")
            .path("E://picture")
            .proxy("http://127.0.0.1:7890");
        let crawler = builder.build();
        assert_eq!(crawler.uuid, "1");
        assert_eq!(crawler.cookie, "2");
        assert_eq!(crawler.path, "E://picture/1");
        assert!(crawler.proxy.is_some());
    }

    #[test]
    fn save_load_config() {
        // back up the config
        let bak = helper::get_config();
        let user = match std::env::var("USERNAME") {
            Ok(user) => user,
            Err(_) => std::env::var("USER").unwrap(),
        };
        let entry = keyring::Entry::new("pixiv_downloader", &user).unwrap();
        entry.delete_password().unwrap();

        // No config test
        let config = helper::get_config();
        let expexcted = serde_json::from_str(
            r#"
            {
                "uuid": "",
                "cookie": "",
                "path": "",
                "proxy": ""
            }
        "#,
        )
        .unwrap();
        assert_eq!(config, expexcted);

        // No path test; Default path expected
        let builder = Crawler::builder().uuid("1").cookie("2").path("").proxy("");
        helper::store_builder(&builder);
        let config = helper::get_config();
        let mut expect = std::collections::HashMap::new();
        expect.insert("uuid".to_owned(), "1".to_owned());
        expect.insert("cookie".to_owned(), "2".to_owned());
        expect.insert("path".to_owned(), helper::download_dir());
        expect.insert("proxy".to_owned(), "".to_owned());
        assert_eq!(config, expect);

        // regular situation test;
        let builder = Crawler::builder()
            .uuid("1")
            .cookie("2")
            .path("D://")
            .proxy("http://127.0.0.1:7890");
        helper::store_builder(&builder);
        let config = helper::get_config();
        let mut expect = std::collections::HashMap::new();
        expect.insert("uuid".to_owned(), "1".to_owned());
        expect.insert("cookie".to_owned(), "2".to_owned());
        expect.insert("path".to_owned(), "D://".to_owned());
        expect.insert("proxy".to_owned(), "http://127.0.0.1:7890".to_owned());
        assert_eq!(config, expect);

        // revert the config after test
        entry.delete_password().unwrap();
        entry
            .set_password(&serde_json::to_string(&bak).unwrap())
            .unwrap();
    }
}
