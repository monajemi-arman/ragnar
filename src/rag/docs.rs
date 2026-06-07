use anyhow::Result;
use notify::{EventKind::Create, RecursiveMode, Watcher, event::CreateKind::File};
use std::{
    fs::create_dir,
    mem::take,
    path::{Path, PathBuf, absolute},
    sync::{Arc, Mutex, mpsc},
};
use tokio::{
    fs::{OpenOptions, read_to_string},
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    sync::mpsc::channel,
};

use crate::{
    app::AppState,
    rag::{embed::embed_and_save, prompt::ask},
};

const MAX_PARAGRAPH_LINES: u16 = 25;

type DocsSeen = Arc<Mutex<Vec<String>>>;

pub async fn watch_folder(state: &AppState) {
    let docs_folder = state.config.docs_folder.clone();

    // Ignore already seen files
    let log_file = Path::new(&state.config.docs_log_file);
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
                // Only keep files, with txt extension
                if !x.is_file() {
                    return false;
                } else {
                    let ext = x.extension().map(|x| x.to_str().unwrap()).unwrap_or("");
                    if "txt" == ext.to_lowercase().as_str() {
                        return true;
                    } else {
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
        process_document(&state, &file_path, &docs_seen_clone, log_file).await;
    }
}

async fn process_document(
    state: &AppState,
    file_path: &PathBuf,
    docs_seen: &DocsSeen,
    log_file: &Path,
) {
    let file_path_str = file_path
        .to_str()
        .expect("failed to do path to str")
        .to_string();
    println!("[...] Processing document: {}", &file_path_str);

    if state.config.prepend_context {
        process_with_prepend_context(&state, file_path).await;
    } else {
        process_without_context(&state, file_path).await;
    }

    // Push to docs_seen and save to docs log file
    docs_seen.lock().unwrap().push(file_path_str.clone());
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file)
        .await
        .expect("failed to open docs log file for append");

    file.write_all((file_path_str.clone() + "\n").as_bytes())
        .await
        .expect("failed to write to docs log file");

    println!(
        "[+] Succesfully added document to database: {}",
        &file_path_str
    );
}

async fn process_with_prepend_context(state: &AppState, file_path: &PathBuf) {
    let file_path_str = file_path.to_str().unwrap();
    let file = OpenOptions::new()
        .read(true)
        .open(&file_path)
        .await
        .expect(&format!("failed to open document: {}", file_path_str));
    let mut reader: Lines<BufReader<tokio::fs::File>> = BufReader::new(file).lines();

    // Purpose: Prepend context while processing buffered file
    // Start reading file, until you have a full buf_now, then put it in buf_full,
    // if buf_full is already occupied, move the content inside buf_full into buf_past,
    // on next iteration, after re-filling buf_now, if buf_full exists, tell LLM to
    // add context from buf_past and buf_now to buf_full, then give the result to embed model.
    let mut buf_lines_count = 0;
    let mut buf_past = String::new(); // Past paragraph
    let mut buf_now = String::new();
    let mut buf_full = String::new(); // Ready to generate embed from
    while let Ok(Some(line)) = reader.next_line().await {
        buf_lines_count += 1;
        buf_now.push_str(line.as_str());
        buf_now.push('\n');

        // blank line = paragraph separator (lines() strips newlines, so "" means was "\n")
        let is_paragraph_break = line.is_empty() || buf_lines_count >= MAX_PARAGRAPH_LINES;

        if is_paragraph_break {
            if !buf_full.is_empty() {
                embed_with_prepend_context(&state, file_path_str, &buf_past, &buf_full, &buf_now)
                    .await
                    .expect("failed to embed document");
                buf_past = take(&mut buf_full);
            }
            buf_full = take(&mut buf_now);
            buf_lines_count = 0;
        }
    }
    // buf_now may have a partial paragraph that never hit the break condition
    if !buf_now.is_empty() {
        if !buf_full.is_empty() {
            // There's a full paragraph waiting; process it with what came before/after
            embed_with_prepend_context(&state, file_path_str, &buf_past, &buf_full, &buf_now)
                .await
                .expect("failed to embed document");
            buf_past = take(&mut buf_full);
        }
        buf_full = take(&mut buf_now);
    }

    // buf_full holds the final paragraph — process it with no "future" context
    if !buf_full.is_empty() {
        embed_with_prepend_context(&state, file_path_str, &buf_past, &buf_full, "")
            .await
            .expect("failed to embed document");
    }
}

async fn embed_with_prepend_context(
    state: &AppState,
    source: &str,
    buf_past: &str,
    buf_full: &str,
    buf_now: &str,
) -> Result<()> {
    let query = format!("Write prepend context from =BEFORE= and =AFTER= paragraphs relevant to the CURRENT paragraph.
        =BEFORE=
        {buf_past}

        =AFTER=
        {buf_now}

        =CURRENT=
        {buf_full} 
    ");

    let prepend_context = ask(&state, &query).await?;
    embed_and_save(&state, source.to_owned(), prepend_context + buf_full).await?;
    Ok(())
}

async fn process_without_context(state: &AppState, file_path: &PathBuf) {
    let file = OpenOptions::new()
        .read(true)
        .open(&file_path)
        .await
        .expect(&format!(
            "failed to open document: {}",
            file_path.to_str().unwrap()
        ));
    let mut reader: Lines<BufReader<tokio::fs::File>> = BufReader::new(file).lines();

    let mut buf_lines_count = 0;
    let mut buf_now = String::new();
    while let Ok(Some(line)) = reader.next_line().await {
        buf_lines_count += 1;
        buf_now.push_str(line.as_str());
        buf_now.push('\n');

        // blank line = paragraph separator (lines() strips newlines, so "" means was "\n")
        let is_paragraph_break = line.is_empty() || buf_lines_count >= MAX_PARAGRAPH_LINES;

        if is_paragraph_break {
            embed_and_save(
                &state,
                file_path.to_str().unwrap().to_string(),
                buf_now.clone(),
            )
            .await
            .expect("failed to do embed and save ");
            buf_now = String::new();
            buf_lines_count = 0;
        }
    }
    // buf_now may have a partial paragraph that never hit the break condition
    if !buf_now.is_empty() {
        embed_and_save(
            &state,
            file_path.to_str().unwrap().to_string(),
            buf_now.clone(),
        )
        .await
        .expect("failed to do embed and save ");
    }
}
