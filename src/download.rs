use crypto_hash::{hex_digest, Algorithm};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use reqwest::{blocking::Client, header::HeaderMap};
use std::{
    fmt::Write,
    fs,
    io::Read,
    path::PathBuf,
    sync::{
        mpsc::{self, Sender},
        Arc, Barrier,
    },
    vec,
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
            temp_folder,
        }
    }

    pub fn run(&self) {
        let headers = get_headers(&self.client, &self.url);
        let file_size = get_file_size(&headers);

        let intervals = into_intervals(file_size, self.threadpool.thread_count() as u64);

        let (tx, rx) = mpsc::channel::<(u16, u64)>();

        let client = Arc::new(self.client.clone());
        let url = Arc::new(self.url.clone());
        let temp_folder = Arc::new(self.temp_folder.clone());

        let mut thread_id: u16 = 1;

        let mut downloaded_bytes = 0;
        let barrier = Arc::new(Barrier::new(self.threadpool.thread_count()+1));
        for interval in intervals {
            let client = Arc::clone(&client);
            let tx = tx.clone();
            let url = Arc::clone(&url);
            let temp_folder = Arc::clone(&temp_folder);
            let barrier = Arc::clone(&barrier);
            self.threadpool.execute(move || {
                let contents =
                    get_bytes_in_range(&client, &url, interval.0, interval.1, tx, thread_id);
                let path = temp_folder.join(&thread_id.to_string());
                fs::write(path, contents).unwrap();
                barrier.wait();
            });

            thread_id += 1;
        }

        let mut threads_progress = vec![0; self.threadpool.thread_count() + 1];

        let pb = ProgressBar::new(file_size as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "[{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
            )
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:1}s", state.eta().as_secs()).unwrap()
            })
            .progress_chars("#>-"),
        );

        while downloaded_bytes < file_size {
            let (id, thread_downloaded_bytes) = rx.recv().unwrap();
            threads_progress[id as usize] = thread_downloaded_bytes;
            downloaded_bytes = threads_progress.iter().sum();
            pb.set_position(downloaded_bytes as u64);
        }

        barrier.wait();

        let mut output = Vec::with_capacity(file_size as usize);

        let mut entries = fs::read_dir(&self.temp_folder)
            .unwrap()
            .map(|res| res.unwrap())
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            let number_a = a.file_name().to_string_lossy().parse::<u8>().unwrap();
            let number_b = b.file_name().to_string_lossy().parse::<u8>().unwrap();
            number_a.cmp(&number_b)
        });

        for entry in entries {
            let filepath = entry.path();
            let file = fs::read(&filepath).unwrap();
            output.extend(file);
            fs::remove_file(filepath).unwrap();
        }

        let file_name = match get_download_name(&headers, &self.url) {
            Some(file_name) => file_name,
            None => hex_digest(Algorithm::MD5, &output),
        };

        fs::write(&file_name, output).unwrap();
        println!("Dowloaded {}", file_name);

    }
}

fn into_intervals(number: u64, interval: u64) -> Vec<(u64, u64)> {
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

fn get_file_size(headers: &HeaderMap) -> u64 {
    //TODO what to do if no file size? go back to single thread probably
    headers
        .get("content-length")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
}

fn get_bytes_in_range(
    client: &Client,
    url: &str,
    start: u64,
    end: u64,
    tx: Sender<(u16, u64)>,
    id: u16,
) -> Vec<u8> {
    let range = format!("bytes={}-{}", start, end);
    let mut response = client
        .get(url)
        .header(reqwest::header::RANGE, range)
        .send()
        .unwrap();
    let mut buffer = Vec::new();
    let mut total_bytes_read = 0;

    loop {
        let mut chunk = [0; 4096];
        let bytes_read = response.read(&mut chunk).unwrap();

        if bytes_read == 0 {
            break;
        }

        total_bytes_read += bytes_read as u64;
        buffer.extend_from_slice(&chunk[..bytes_read]);
        tx.send((id, total_bytes_read)).unwrap();
    }

    buffer
}

fn get_download_name(headers: &HeaderMap, url: &str) -> Option<String> {
    let content_disposition = headers.get("Content-Disposition");
    if let Some(cd) = content_disposition {
        if let Ok(cd_string) = cd.to_str() {
            if let Some(filename) = cd_string.split("filename=").nth(1) {
                return Some(filename.to_owned());
            }
        }
    }
    let parts: Vec<&str> = url.split('/').collect();
    if let Some(filename) = parts.last() {
        return Some(filename.to_string());
    } else {
        return None;
    }
}

fn get_headers(client: &Client, url: &str) -> HeaderMap {
    let binding = client.get(url).send().unwrap();
    binding.headers().to_owned()
}
