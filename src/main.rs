#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use std::sync::Mutex;

use anyhow::Result;
use clap::{value_t, App, Arg};
use directories::UserDirs;
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use tempfile::tempdir;
use walkdir::{DirEntry, WalkDir};
use which::which;

use notify::event::{CreateKind, EventKind::*, ModifyKind, RenameMode};
use notify::{RecursiveMode, Watcher};

use actix_cors::Cors;
use actix_files as fs;
use actix_files::NamedFile;
use actix_web::{web, HttpServer, Responder};

static ARGS: OnceCell<Args> = OnceCell::new();
static PATHS: Lazy<Mutex<Vec<PathBuf>>> = Lazy::new(|| Mutex::new(vec![]));

#[derive(Debug, Clone)]
struct Args {
  pub recursive: bool,
  pub path: PathBuf,
  pub depth: Option<usize>,
  pub thumbnail_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
struct MetaData {
  pub width: usize,
  pub height: usize,
  pub duration: String,
  pub bit_rate: String,
}

fn path_is_thumbnail(path: &Path) -> bool {
  path.display().to_string().contains("thumbnail")
}

fn validate_files(entry: &DirEntry) -> bool {
  if entry.file_type().is_dir() {
    return false;
  }

  let re = Regex::new(r".*(?i:jpe?g|png|gif|mp4|webm)$").unwrap();

  entry
    .file_name()
    .to_str()
    .map(|s| re.is_match(s))
    .unwrap_or(false)
}

async fn get_paths() -> impl Responder {
  let args = ARGS.get().unwrap();
  let normailized_paths = PATHS
    .lock()
    .unwrap()
    .iter()
    .filter(|p| !path_is_thumbnail(p))
    .map(|p| PathBuf::from(p.strip_prefix(&args.path).unwrap()))
    .collect::<Vec<_>>();
  web::Json(normailized_paths)
}

async fn exists_file_on_server(file: web::Path<String>) -> impl Responder {
  let args = ARGS.get().unwrap();

  for p in WalkDir::new(&args.thumbnail_dir) {
    if p.unwrap().path().ends_with(file.to_owned()) {
      return web::Json(true);
    }
  }

  web::Json(false)
}

async fn index() -> impl Responder {
  NamedFile::open("static/index.html")
}

fn read_metadata(path: &Path) -> Result<MetaData> {
  let out = Command::new("ffprobe")
    .args(&[
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=width,height,duration,bit_rate",
      "-of",
      "json",
      &path.display().to_string(),
    ])
    .output()?;

  let meta_data: Value = serde_json::from_str(str::from_utf8(&out.stdout)?)?;
  Ok(serde_json::from_value(meta_data["streams"][0].clone())?)
}

fn transcode() {
  std::thread::spawn(move || {
    let g_args = ARGS.get().unwrap();

    let threads = num_cpus::get() / 4;
    let threads = if threads > 0 { threads } else { 1 };

    for path in PATHS
      .lock()
      .unwrap()
      .iter()
      .filter(|p| !path_is_thumbnail(p))
    {
      if path.extension().unwrap() == "mp4" {
        let meta_data = read_metadata(path).unwrap_or(MetaData {
          width: 320,
          height: 240,
          duration: "15.0".to_owned(),
          bit_rate: "1010000".to_owned(),
        });

        let parse_duration = |duration: &str| -> usize {
          let mut splitted_duration = duration.split('.').into_iter();
          let number = splitted_duration.next().unwrap();
          number.parse::<usize>().unwrap()
        };

        println!("Transcoding \"{}\"", path.display());

        let mut args = [
          "-threads".to_owned(),
          format!("{}", threads),
          "-i".to_owned(),
          path.display().to_string(),
          "-ss".to_owned(),
          "00:00:00".to_owned(),
        ]
        .to_vec();

        if parse_duration(&meta_data.duration) > 15 {
          args.extend(["-t".to_owned(), "00:00:15".to_owned()].to_vec());
        }

        if meta_data.width > 320 || meta_data.height > 240 {
          args.extend(["-vf".to_owned(), "scale=320:240".to_owned()].to_vec());
        }

        if meta_data.bit_rate.parse::<usize>().unwrap() > 1_010_000 {
          args.extend(["-b:v".to_owned(), "1M".to_owned()].to_vec());
        }

        let thumbnail = format!(
          "{}_thumbnail.{}",
          path.file_stem().unwrap().to_string_lossy(),
          path.extension().unwrap().to_string_lossy()
        );

        Command::new("ffmpeg")
          .args(&args)
          .arg(g_args.thumbnail_dir.join(&thumbnail))
          .output()
          .expect("Failed to transcode video");
      }
    }
  });
}

fn walk_paths(recursive: bool, depth: Option<usize>, path: &Path) -> Vec<PathBuf> {
  let walker = if recursive {
    WalkDir::new(&path)
  } else {
    WalkDir::new(&path).max_depth(1)
  };

  let walker = if let Some(depth) = depth {
    WalkDir::new(&path).max_depth(depth)
  } else {
    walker
  };

  walker
    .into_iter()
    .filter_map(Result::ok)
    .filter(|e| validate_files(e))
    .map(|e| PathBuf::from(e.path()))
    .collect::<Vec<_>>()
}

#[actix_rt::main]
async fn main() -> Result<()> {
  std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
  env_logger::init();

  let matches = App::new("swmv")
    .version("0.1.0")
    .author("Michael Kaltschmid <kaltschmidmichael@gmail.com>")
    .about("Simple Imgur inspired media viewer.")
    .arg(
      Arg::with_name("path")
        .short("p")
        .long("path")
        .help("The path to search for images and videos")
        .required(false)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("recursive")
        .short("r")
        .long("recursive")
        .help("Searches for images and videos recursively")
        .required(false)
        .takes_value(false),
    )
    .arg(
      Arg::with_name("depth")
        .short("d")
        .long("depth")
        .help("Specifies the maximum depth of entries yield by the iterator")
        .required(false)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("thumbnail")
        .short("t")
        .long("thumbnail")
        .help("Creates short clips for the videos in the viewer. Requires FFMPEG and can be CPU intensive.")
        .required(false)
        .takes_value(false),
    )
    .arg(
      Arg::with_name("delete")
        .long("delete")
        .help("Deletes thumbnails.")
        .required(false)
        .takes_value(false),
    )
    .get_matches();

  let path = matches.value_of("path").map(PathBuf::from).unwrap_or_else(||
    UserDirs::new()
      .expect("Cannot get user directories.")
      .picture_dir()
      .map(PathBuf::from)
      .expect("Cannot find picture directory. Specify a valid path.")
  );

  let recursive = matches.is_present("recursive");

  let depth = if matches.is_present("depth") {
    Some(value_t!(matches.value_of("depth"), usize).unwrap_or_else(|e| e.exit()))
  } else {
    None
  };

  let temp_dir = tempdir().unwrap();
  let temp_dir = PathBuf::from(temp_dir.path());

  let args = Args {
    recursive: recursive,
    path: path.clone(),
    depth: depth,
    thumbnail_dir: temp_dir.clone(),
  };

  ARGS.set(args).unwrap();

  for walk_path in walk_paths(recursive, depth, &path) {
    PATHS.lock().unwrap().push(walk_path);
  }

  if matches.is_present("thumbnail") {
    if which("ffmpeg").is_ok() && which("ffprobe").is_ok() {
      transcode();
    } else {
      error!("\"ffmpeg\" and \"ffprobe\" need to be installed for transcode support.");
    }
  } else if matches.is_present("delete") {
    for path in PATHS
      .lock()
      .unwrap()
      .iter()
      .filter(|p| path_is_thumbnail(p))
    {
      remove_file(path)?;
    }
  }

  let args = ARGS.get().unwrap();

  let add_paths = |paths: &Vec<PathBuf>| {
    for path in paths.iter() {
      if !PATHS.lock().unwrap().contains(path) {
        PATHS.lock().unwrap().push(path.clone());
      }
    }
  };

  // Watch for changes and update paths.
  let mut watcher =
    notify::recommended_watcher(move |res: Result<notify::Event, notify::Error>| match res {
      Ok(event) => match event.kind {
        Create(kind) => {
          if kind == CreateKind::File {
            add_paths(&event.paths);
          }

          if kind == CreateKind::Folder {
            for p in event.paths {
              for entry in WalkDir::new(p) {
                if let Ok(entry) = entry {
                  if entry.path().is_file()
                    && !PATHS.lock().unwrap().contains(&entry.path().to_path_buf())
                  {
                    PATHS
                      .lock()
                      .unwrap()
                      .push(entry.path().to_path_buf().clone());
                  }
                }
              }
            }
          }
        }
        Modify(meta) => match meta {
          ModifyKind::Name(name) => {
            if name == RenameMode::From {
              for path in event.paths.iter() {
                PATHS
                  .lock()
                  .unwrap()
                  .retain(|p| !p.starts_with(path) || p != path);
              }
            }

            if name == RenameMode::To {
              add_paths(&event.paths);
            }
          }
          _ => (),
        },
        _ => (),
      },
      Err(e) => println!("watch error: {:?}", e),
    })?;

  if args.recursive {
    watcher.watch(&args.path, RecursiveMode::Recursive)?;
  } else {
    watcher.watch(&args.path, RecursiveMode::NonRecursive)?;
  }

  HttpServer::new(move || {
    actix_web::App::new()
      .wrap(Cors::default())
      .service(fs::Files::new("/static", "static").index_file("index.html"))
      .service(fs::Files::new("/media", &path).show_files_listing())
      .service(fs::Files::new("/thumbnails", &temp_dir).show_files_listing())
      .route("/", web::get().to(index))
      .route("/paths/", web::get().to(get_paths))
      .route("/file/exists/{file}", web::get().to(exists_file_on_server))
  })
  .workers(2)
  .bind(("0.0.0.0", 8288))?
  .run()
  .await?;

  // ffmpeg -hwaccel cuvid -c:v h264_cuvid -resize 320x240 -i vid -ss 00:00:00 -t 00:00:20 -c:a copy -c:v h264_nv enc -b:v 3M output.mp4

  Ok(())
}
