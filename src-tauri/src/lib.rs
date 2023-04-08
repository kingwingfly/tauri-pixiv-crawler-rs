use std::pin::Pin;

use futures::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod helper {
    pub fn create_rt() -> tokio::runtime::Runtime {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
    }
}

pub struct Crawler {}

impl Crawler {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&self, num: usize) {
        for i in (0..=num).rev() {
            println!("{i}");
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

pub struct TaskMng {
    tx: mpsc::Sender<Message>,
    total: AtomicUsize,
    finished: Arc<AtomicUsize>,
    tx_back: mpsc::Sender<bool>,
}

struct Task {
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
    tx_back: Option<tokio::sync::mpsc::Sender<bool>>,
}

enum Message {
    Job(Task),
    Terminate,
}

impl TaskMng {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<Message>(16);
        let (tx_back, mut rx_back) = mpsc::channel(16);
        let finished = Arc::new(AtomicUsize::new(0));
        let finished_clone = finished.clone();
        std::thread::spawn(move || {
            let rt = helper::create_rt();
            rt.block_on(async move {
                loop {
                    tokio::select! {
                        Some(msg) = rx.recv() => {
                            match msg {
                                Message::Job(task) => {
                                    tokio::spawn(async {
                                        task.task.await;
                                        if let Some(tx_back) = task.tx_back {
                                            tx_back.send(true).await.unwrap();
                                        }
                                    });
                                }
                                Message::Terminate => break,
                            };
                        }
                        Some(res) = rx_back.recv() => {
                            if res {
                                finished_clone.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                    }
                }
                // println!("Shut down")
            })
        });

        Self {
            tx,
            total: AtomicUsize::new(0),
            finished,
            tx_back,
        }
    }

    pub fn spawn_task<F>(&self, task: F)
    where
        F: Future<Output = ()> + Send + 'static,
        F::Output: Send,
    {
        let tx_back = self.tx_back.clone();
        let task = Task {
            task: Box::pin(task),
            tx_back: Some(tx_back),
        };
        let msg = Message::Job(task);
        match self.tx.blocking_send(msg) {
            Ok(()) => {
                self.total.fetch_add(1, Ordering::SeqCst);
            }
            Err(_) => panic!("The shared runtime has shut down."),
        };
    }

    pub fn process(&self) -> String {
        format!(
            "{}/{}",
            self.finished.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed)
        )
    }

    pub fn shutdown(&self) {
        self.tx.blocking_send(Message::Terminate).ok().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taskmng_test() {
        let taskmng = TaskMng::new();
        taskmng.spawn_task(async {});
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert_eq!(taskmng.process(), "1/1");
        taskmng.spawn_task(async {});
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert_eq!(taskmng.process(), "2/2");
    }

    #[test]
    #[should_panic(expected = "The shared runtime has shut down.")]
    fn shut_down_test() {
        let taskmng = TaskMng::new();
        taskmng.shutdown();
        std::thread::sleep(std::time::Duration::from_secs(1));
        taskmng.spawn_task(async {});
    }
}
