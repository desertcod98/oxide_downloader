use reqwest::blocking::Client;
use threadpool::ThreadPool;
use std::{
    fs,
    sync::{mpsc, Arc, Mutex},
};

pub struct Download {
    url: String,
    threadpool: ThreadPool,
    client: Client,
}

impl Download {
    pub fn new(url: &str, n_threads: usize) -> Self {
        Download {
            url: url.to_owned(),
            threadpool: ThreadPool::new(n_threads),
            client: Client::new(),
        }
    }

    pub fn run(&self) {
        let intervals = into_intervals(
            get_file_size(&self.client, &self.url),
            self.threadpool.thread_count() as u32,
        );

        let done_counter = Arc::new(Mutex::new(0));

        let (tx, rx) = mpsc::channel::<()>();

        let client = Arc::new(self.client.clone());
        let url = Arc::new(self.url.clone());

        for interval in intervals {
            let client = Arc::clone(&client);
            let tx = tx.clone();
            let url = Arc::clone(&url);

            self.threadpool.execute(move || {
                let contents = get_bytes_in_range(&client, &url, interval.0, interval.1);
                fs::write(format!("./result/{}", interval.0), contents).unwrap();
                tx.send(()).unwrap();
            });
        }

        while *done_counter.lock().unwrap() < self.threadpool.thread_count() {
            rx.recv().unwrap();
            *done_counter.lock().unwrap() += 1;
        }

        println!("done");

        let filesize = get_file_size(&client, &url);
        let mut output = Vec::with_capacity(filesize as usize);

        for file in fs::read_dir("./result").unwrap() {
            output.extend(fs::read(file.unwrap().path()).unwrap());
        }

        fs::write("output.mp4", output).unwrap();
    }
}

fn into_intervals(number: u32, interval: u32) -> Vec<(u32, u32)> {
    let interval_size = (number + 1) / interval;
    let mut intervals = Vec::new();
    let mut current_start = 0;
    for i in 0..interval {
        let current_end = if i == interval - 1 {
            number
        } else {
            current_start + interval_size - 1
        };
        intervals.push((current_start, current_end));
        current_start = current_end + 1;
    }
    intervals
}

fn get_file_size(client: &Client, url: &str) -> u32 {
    client
        .get(url)
        .send()
        .unwrap()
        .headers()
        .get("content-length")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u32>()
        .unwrap()
}

fn get_bytes_in_range(client: &Client, url: &str, start: u32, end: u32) -> Vec<u8> {
    let range = format!("bytes={}-{}", start, end);
    client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec()
}
