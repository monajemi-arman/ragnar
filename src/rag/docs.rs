use std::{
    fs::create_dir,
    path::{Path, PathBuf, absolute},
    sync::{Arc, Mutex, mpsc},
};

use notify::{EventKind::Create, RecursiveMode, Watcher, event::CreateKind::File};
use tokio::{
    fs::{OpenOptions, read_to_string},
    io::AsyncWriteExt,
    sync::mpsc::channel,
};

use crate::Config;

type DocsSeen = Arc<Mutex<Vec<String>>>;

pub async fn watch_folder(config: &Config) {
    let docs_folder = config.docs_folder.clone();

    // Ignore already seen files
    let log_file = Path::new(&config.docs_log_file);
    let docs_seen: DocsSeen;
    if log_file.is_file() {
        docs_seen = Arc::new(Mutex::new(
            read_to_string(log_file)
                .await
                .expect("failed to read docs log file")
                .lines()
                .map(|x| x.to_string())
                .collect(),
        ));
    } else {
        docs_seen = Arc::new(Mutex::new(vec![]));
    }

    // Sync channel is for the blocking tokio task due to sync nature of notify
    let (sync_tx, sync_rx) = mpsc::channel();
    let (async_tx, mut async_rx) = channel(100); // sync -> async channel
    let mut watcher = notify::recommended_watcher(sync_tx).expect("failed to get watcher");
    let docs_seen_clone = Arc::clone(&docs_seen);

    tokio::task::spawn_blocking(move || {
        let docs_folder = Path::new(&docs_folder);
        if !docs_folder.exists() {
            create_dir(docs_folder).expect("failed to create docs folder")
        }

        // Initial check of file list
        let initial_files: Vec<_> = walkdir::WalkDir::new(docs_folder)
            .into_iter()
            .map(|x| {
                absolute(
                    x.expect("failed to list docs folder")
                        .into_path()
                        .to_owned(),
                )
                .expect("failed to get absolute path from relative in intial files listing")
            })
            .filter(|x| {
                // Only keep files, with pdf / txt extension 
                if !x.is_file() {
                    return false;
                } else {
                    let ext = x.extension().map(|x| x.to_str().unwrap()).unwrap_or("");
                    if ["pdf", "txt"].contains(&ext.to_lowercase().as_str()) {
                        return true;
                    }
                    else {
                        return false;
                    }
                }
            })
            .collect();

        for file_path in initial_files {
            if !docs_seen.lock().unwrap().contains(
                &file_path
                    .to_str()
                    .expect("failed to do path to str")
                    .to_string(),
            ) {
                if async_tx.blocking_send(file_path).is_err() {
                    break;
                }
            }
        }

        // Watch new files
        let _watcher = watcher
            .watch(docs_folder, RecursiveMode::Recursive)
            .expect("failed to watch");

        while let Ok(event) = sync_rx.recv() {
            if let Ok(mut event) = event {
                if event.kind == Create(File) {
                    if let Some(file_path) = event.paths.pop() {
                        if async_tx.blocking_send(file_path).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    while let Some(file_path) = async_rx.recv().await {
        process_document(&file_path, &docs_seen_clone, log_file).await;
    }
}

async fn process_document(file_path: &PathBuf, docs_seen: &DocsSeen, log_file: &Path) {
    println!("[...] Processing document: {}", file_path.to_str().unwrap());

    // Push to docs_seen and save to docs log file
    let file_path_str = file_path
        .to_str()
        .expect("failed to do path to str")
        .to_string();
    docs_seen.lock().unwrap().push(file_path_str.clone());
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file)
        .await
        .expect("failed to open docs log file for append");

    file.write_all((file_path_str + "\n").as_bytes())
        .await
        .expect("failed to write to docs log file");

    println!(
        "[+] Succesfully added document to database: {}",
        file_path.to_str().unwrap()
    );
}

fn clean_pdf_text(raw: &str) -> String {
    raw.lines()
        .map(|line| line.trim())
        .fold(String::new(), |mut acc, line| {
            if line.is_empty() {
                // Blank line = paragraph boundary, preserve it
                acc.push_str("\n\n");
            } else if acc.ends_with('-') {
                // Dehyphenate: "proces-\nsing" → "processing"
                acc.pop();
                acc.push_str(line);
            } else if acc.ends_with('\n') {
                // Soft line break within same paragraph — join with space
                acc.push(' ');
                acc.push_str(line);
            } else {
                acc.push_str(line);
            }
            acc
        })
}
