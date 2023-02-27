use reqwest::{self, blocking::Client};
use std::{
    fs,
    sync::{mpsc, Arc},
};
use threadpool::ThreadPool;

fn main() {
    let n_threads = 16;
    let client = Arc::new(Client::new());

    let threadpool = ThreadPool::new(n_threads);
    let url = "https://getsamplefiles.com/download/mp4/sample-1.mp4";

    let mut done_counter = 0;
    let intervals = into_intervals(get_file_size(&client, url), n_threads as u32);

    let (tx, rx) = mpsc::channel::<()>();

    for interval in intervals {
        let client = Arc::clone(&client);
        let tx = tx.clone();
        threadpool.execute(move || {
            let contents = get_bytes_in_range(&client, &url, interval.0, interval.1);
            fs::write(format!("./result/{}", interval.0), contents).unwrap();
            tx.send(()).unwrap();
        });
    }

    while done_counter < n_threads {
        rx.recv().unwrap();
        done_counter += 1;
    }

    println!("done");

    let filesize = get_file_size(&client, url);
    let mut output = Vec::with_capacity(filesize as usize);

    for file in fs::read_dir("./result").unwrap() {
        output.extend(fs::read(file.unwrap().path()).unwrap());
    }

    fs::write("output.mp4", output).unwrap();
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
