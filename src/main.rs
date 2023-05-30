use config::{Config, File};
use directories::ProjectDirs;
use std::collections::HashMap;
use std::fs::DirBuilder;
use std::{
    env,
    path::{Path, PathBuf},
};
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



    let config = Config::builder()
        .add_source(File::with_name("config.yaml"))
        .build()
        .unwrap();

    let temp_folder: PathBuf = match config.get::<String>("temp_folder") {
        Ok(temp_folder) => {
            let path = PathBuf::from(temp_folder);
            if path.exists() {
                path
            } else {
                ProjectDirs::from("", "desertcod98", "oxide")
                    .unwrap()
                    .cache_dir()
                    .to_path_buf()
            }
        }
        Err(_) => {
            let current_dir = env::current_dir().expect("Couldn't get current directory");
            let mut result_path = PathBuf::from(current_dir);
            result_path.push("result");
            println!("Using default temp folder ({}), modify temp_folder propriety in config.yaml to change it",result_path.to_string_lossy());
            PathBuf::from(result_path)
        }
    };

    //create temp folder if it doesn't exist
    if !temp_folder.exists() {
        let mut dir_builder = DirBuilder::new();
        dir_builder.recursive(true);
        dir_builder.create(&temp_folder).unwrap();
    }

    let download = Download::new(url, n_threads, temp_folder);

    download.run();
}
