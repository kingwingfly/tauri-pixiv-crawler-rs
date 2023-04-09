use std::pin::Pin;

use futures::Future;
use reqwest::{
    header::{self, HeaderMap},
    Client, Proxy,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub struct Crawler {
    uuid: String,
    cookie: String,
    path: String,
    proxy: Proxy,
    tk_mng: TaskMng,
}

impl Crawler {
    pub fn new(uuid: &str, cookie: &str, path: &str, proxy: &str) -> Self {
        let proxy = Proxy::all(proxy).unwrap();
        let path = format!("{}/{}", path, uuid);
        let tk_mng = TaskMng::new();
        Self {
            uuid: uuid.into(),
            cookie: cookie.into(),
            path,
            proxy,
            tk_mng,
        }
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
        let client = Client::builder().proxy(self.proxy.clone()).build().unwrap();
        let resp = client.get(&url).send().await?;
        let content = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        let illus: Vec<String> = v.as_object().unwrap()["body"].as_object().unwrap()["illusts"]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.to_owned())
            .collect();
        Ok(illus)
    }

    async fn step2(
        illu_ajax: String,
        proxy: Proxy,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let client = Client::builder().proxy(proxy).build().unwrap();
        let resp = client.get(illu_ajax).send().await?;
        let content = resp.text().await?;
        let v: serde_json::Value = serde_json::from_str(&content).unwrap();
        let ori_urls: Vec<String> = v.as_object().unwrap()["body"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                i.as_object().unwrap()["urls"].as_object().unwrap()["original"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        Ok(ori_urls)
    }

    async fn step3(ori_url: String, headers: HeaderMap, proxy: Proxy, path: String) {
        let name = ori_url.split('/').last().unwrap();
        let path = format!("{}/{}", path, name);
        if fs::try_exists(&path).await.unwrap() {
            return;
        }
        println!("Doing: {}", ori_url);
        loop {
            let client = Client::builder().proxy(proxy.clone()).build().unwrap();
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
            file.write(&content).await.unwrap();
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
    pub fn create_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
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
        let cookie = "_ga_75BBYNYN9J=GS1.1.1680857407.49.1.1680858856.0.0.0; _fbp=fb.1.1673501034604.1950696989; __cf_bm=dKKy3vd6I7cgQ0hgZe_IwggJcmOOnxn6ERkSX7xjUpM-1680858782-0-AUi3gQdJCkf0Q5qepI05M/462X3A3mNOc3jWmrACHn6/xCTsfLD3uS2XTtjzNohmDRuthevUVItR529MvkcHuYXuwcay1IjtHI1874ILxOsa3yYI0Cq+CmFFeU/WX5Gmw5peWUOGKQdB/O/slE9F7O5+HPd76iTEs2LtteEcHrRKcXD297aHPGPbqOeBp873qQ==; _ga=GA1.1.625693455.1673501032; PHPSESSID=33192056_jFm1Tvb7uPEaBHSAOlMw7JZauSY6bO8a; b_type=0; privacy_policy_notification=0; _im_vid=01GPJ6ZA2SCTVEMRRZW4JAZ1K6; c_type=21; QSI_S_ZN_5hF4My7Ad6VNNAi=v:0:0; p_b_type=1; tag_view_ranking=0xsDLqCEW6~KMpT0re7Sq~SIpXPnQ53M~rMbyd5uAhj~Lt-oEicbBr~_EOd7bsGyl~RTJMXD26Ak~UgLGWGysi-~AoKfsFwwdu~Vt3Tl4Tkoa~BSlt10mdnm~Tcn3gevBtQ~HHxwTpn5dx~jH0uD88V6F~6cIiIlKj-K~wm006gFVAz~CAhAmfRBQs~vP6kTD-0Xd~Bd2L9ZBE8q~faHcYIP1U0~ziiAzr_h04~Ie2c51_4Sp~l6cxseQIBN~O8u4lwX_cT~2VnbKugRTI~RiZgkjd5Cv~26-Sd3V3Py~D9BseuUB5Z~lW3PDRuOC9~CiSfl_AE0h~e2yEFDVXjZ~YI8LmI20qW~EUwzYuPRbU~azESOjmQSV~7dpqkQl8TH~oDcj90OVdf~GNcgbuT3T-~ETjPkL0e6r~OT4SuGenFI~kP7msdIeEU~RXk9hi7kn_~QOc7RQXB8U~LfyX5eCTtL~lH5YZxnbfC~aKhT3n4RHZ~kGYw4gQ11Z~tgP8r-gOe_~KN7uxuR89w~jm40SVtdHx~K8esoIs2eW~ti_E1boC1J~zIv0cf5VVk~mLrrjwTHBm~alQb7gJxOf~pnCQRVigpy~J4-uQ7g8Dw~G_vM51w8ml~HMU_P-aYJG~PgQgNyh9aH~zyKU3Q5L4C~PwDMGzD6xn~BtXd1-LPRH~kwxbx_VxB2~1NmdBLOfGO~os5VB0oZyX~o7hvUrSGDN~2QTW_H5tVX~eVxus64GZU~H9o4sUN8F1~pzzjRSV6ZO~Qri62qoiFF~fg8EOt4owo~7fCik7KLYi~ION4v6ZHqO~9Gbahmahac~9wN-K8_crj~pzZvureUki~JJ4D2-VDRE~iFcW6hPGPU~EGefOqA6KB~aOGQhsapNP~LEmyJ-RN72~j6sKkcbNBV~MUQoS0sfqG~0J097IdBNd~zb1N_JZSZu~BA8VCLPrP0~zd0kMkvoqd~Cpw-lbF1eB~1HSjrqSB3U~_pwIgrV8TB~uu4WDPyt4x~Z4hQZu-rU-~FPR_bzUx-I~-RR2Rsko5M~ZXFMxANDG_~JXmGXDx4tL~eInvgwdvwj~gUIg7nrQgl~02lg2Bq_mf; __utma=235335808.625693455.1673501032.1676024046.1676024046.1; __utmv=235335808.|3=plan=normal=1^6=user_id=33192056=1^11=lang=zh=1; __utmz=235335808.1676024046.1.1.utmcsr=(direct)|utmccn=(direct)|utmcmd=(none); adr_id=Q1VxuKbvVTCs14KBezq5vzQebRnn4adNbdw2JETMs2XDegoP; _ts_yjad=1674656290200; _ga_1Q464JT0V2=GS1.1.1674656289.1.0.1674656289.0.0.0; _ga_MZ1NL4PHH0=GS1.1.1674503773.2.0.1674503775.0.0.0; first_visit_datetime=2023-01-23+20%3A29%3A05; _gcl_au=1.1.27740088.1674473256; pt_60er4xix=uid=Wp3sTmzAFRxIqQLKMDIJLw&nid=0&vid=Z/PFHXF9ngOrUL7ninxXmQ&vn=2&pvn=1&sact=1674465855673&to_flag=0&pl=JWYJn-k8h/7K2Bjkw-IBJA*pt*1674465849805; a_type=1; _im_uid.3929=b.c66866c9851b7f8e; login_ever=yes; privacy_policy_agreement=5; p_ab_d_id=1824640068; p_ab_id=0; p_ab_id_2=3; first_visit_datetime_pc=2022-10-01+04%3A16%3A48; yuid_b=g3FAREA";
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
}
