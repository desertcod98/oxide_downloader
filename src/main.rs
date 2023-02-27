use std::env;

mod download;
use download::Download;

fn main() {
    let args: Vec<String> = env::args().collect();

    let url = args.get(1).expect("Insert URL!");
    let n_threads = args
        .get(2)
        .expect("Insert number of threads!")
        .parse::<usize>()
        .unwrap();

    let download = Download::new(url, n_threads);

    download.run();
}
