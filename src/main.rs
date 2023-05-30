use config::{Config, File};
use directories::ProjectDirs;
use std::fs::{DirBuilder};
use std::io::Write;
use std::{env, path::PathBuf};
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
        .build();

    let temp_folder = match config {
        Ok(config) => match config.get::<String>("temp_folder") {
            Ok(temp_folder) => {
                let path = PathBuf::from(temp_folder);
                if path.exists() {
                    path
                } else {
                    let cache_directory = get_cache_directory();
                    println!("temp_folder in config.yaml ({}) not found, using default temp folder {}",path.to_string_lossy(), cache_directory.to_string_lossy());
                    cache_directory
                }
            }
            Err(_) => {
                println!("temp_folder not set in config.yaml");
                let cache_directory = get_cache_directory();
                println!("Using default temp folder ({}), modify temp_folder propriety in config.yaml to change it",cache_directory.to_string_lossy());
                cache_directory
            }
        },
        Err(_) => {
            println!("Creating config.yaml file");
            let file = std::fs::File::create("config.yaml");
            if let Ok(mut file) = file {
                file.write_all(b"temp_folder : \"\"")
                    .expect("Could not write config file");
            }
            get_cache_directory()
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

fn get_cache_directory() -> PathBuf {
    ProjectDirs::from("", "desertcod98", "oxide")
        .unwrap()
        .cache_dir()
        .to_path_buf()
}
