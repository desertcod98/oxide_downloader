use reqwest::{blocking::Client, header::HeaderMap};
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
        
        let headers = get_headers(&self.client, &self.url);
        let file_size = get_file_size(&headers);
        

        let intervals = into_intervals(
            file_size,
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

        let mut output = Vec::with_capacity(file_size as usize);

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

        let file_name = get_download_name(&headers, &self.url);
        
        fs::write(file_name, output).unwrap();
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

fn get_file_size(headers: &HeaderMap) -> u32 {
    //TODO what to do if no file size? go back to single thread probably
    headers
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

fn get_download_name(headers: &HeaderMap, url: &str) -> String{
    let content_disposition = headers.get("Content-Disposition");
    if let Some(cd) = content_disposition {
        if let Ok(cd_string) = cd.to_str(){
            if let Some(filename) = cd_string.split("filename=").nth(1){
                return filename.to_owned();
            }
        }
    }
    let parts: Vec<&str> = url.split('/').collect();
    if let Some(filename) = parts.last(){
        return filename.to_string();
    }else{
        return "UNKNOWN".to_owned();
    }
    
}

fn get_headers(client: &Client, url: &str) -> HeaderMap{
    let binding = client
            .get(url)
            .send()
            .unwrap();
        binding
            .headers().to_owned()
}