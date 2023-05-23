use reqwest::blocking::Client;
use std::{
    fs,
    path::PathBuf,
    sync::{mpsc, Arc, Mutex}, io::Read,
};
use threadpool::ThreadPool;

pub struct Download {
    url: String,
    threadpool: ThreadPool,
    client: Client,
    temp_folder: PathBuf,
}

impl Download {
    pub fn new(url: &str, n_threads: usize, temp_folder: PathBuf) -> Self {
        Download {
            url: url.to_owned(),
            threadpool: ThreadPool::new(n_threads),
            client: Client::new(),
            temp_folder
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
        let temp_folder = Arc::new(self.temp_folder.clone());
        
        let mut temp_file_counter : u8 = 1;

        for interval in intervals {
            let client = Arc::clone(&client);
            let tx = tx.clone();
            let url = Arc::clone(&url);
            let temp_folder = Arc::clone(&temp_folder);

            self.threadpool.execute(move || {
                let contents = get_bytes_in_range(&client, &url, interval.0, interval.1);
                let path = temp_folder.join(&temp_file_counter.to_string());
                fs::write(path, contents).unwrap();
                tx.send(()).unwrap();
            });

            temp_file_counter += 1;
        }

        while *done_counter.lock().unwrap() < self.threadpool.thread_count() {
            rx.recv().unwrap();
            *done_counter.lock().unwrap() += 1;
        }

        println!("done");

        let filesize = get_file_size(&client, &url);
        let mut output = Vec::with_capacity(filesize as usize);

        let mut entries = fs::read_dir(&self.temp_folder).unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
                let number_a = a.file_name().to_string_lossy().parse::<u8>().unwrap();
                let number_b = b.file_name().to_string_lossy().parse::<u8>().unwrap();
                number_a.cmp(&number_b)
            }
        );

        for entry in entries{
            let filepath = entry.path(); 
            let file = fs::read(&filepath).unwrap();
            output.extend(file);
            fs::remove_file(filepath).unwrap();
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
