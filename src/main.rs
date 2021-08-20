#[macro_use]
extern crate log;
extern crate env_logger;

mod args;
mod transcode;
mod watcher;

use crate::args::get_args;
use crate::transcode::transcode;
use crate::watcher::watch;

use std::fs::remove_file;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use anyhow::Result;
use clap::value_t;
use directories::UserDirs;
use once_cell::sync::{Lazy, OnceCell};
use regex::Regex;
use tempfile::tempdir;
use walkdir::{DirEntry, WalkDir};
use which::which;

use actix_cors::Cors;
use actix_files as fs;
use actix_files::NamedFile;
use actix_web::{web, HttpServer, Responder};

pub static ARGS: OnceCell<Args> = OnceCell::new();
pub static PATHS: Lazy<RwLock<Vec<PathBuf>>> = Lazy::new(|| RwLock::new(vec![]));

#[derive(Debug, Clone)]
pub struct Args {
  pub recursive: bool,
  pub path: PathBuf,
  pub depth: Option<usize>,
  pub thumbnail_dir: PathBuf,
  pub cuda: bool,
}

pub fn path_is_thumbnail(path: &Path) -> bool {
  path.to_string_lossy().contains("thumbnail")
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

async fn get_paths() -> impl Responder {
  let args = ARGS.get().unwrap();

  let normailized_paths = PATHS
    .read()
    .unwrap()
    .iter()
    .filter(|p| !path_is_thumbnail(p))
    .map(|p| PathBuf::from(p.strip_prefix(&args.path).unwrap()))
    .collect::<Vec<_>>();

  web::Json(normailized_paths)
}

async fn index() -> impl Responder {
  NamedFile::open("static/index.html")
}

#[actix_rt::main]
async fn main() -> Result<()> {
  std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
  env_logger::init();

  let matches = get_args().get_matches();

  let path = matches
    .value_of("path")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      UserDirs::new()
        .expect("Cannot get user directories.")
        .picture_dir()
        .map(PathBuf::from)
        .expect("Cannot find picture directory. Specify a valid path.")
    });

  let recursive = matches.is_present("recursive");

  let depth = if matches.is_present("depth") {
    Some(value_t!(matches.value_of("depth"), usize).unwrap_or_else(|e| e.exit()))
  } else {
    None
  };

  let cuda = matches.is_present("cuda");

  rayon::ThreadPoolBuilder::new()
    .num_threads(if cuda { 3 } else { 1 })
    .build_global()
    .unwrap();

  let temp_dir = tempdir().unwrap();
  let temp_dir = PathBuf::from(temp_dir.path());

  let args = Args {
    recursive,
    path: path.clone(),
    depth,
    thumbnail_dir: temp_dir.clone(),
    cuda,
  };

  ARGS.set(args).unwrap();

  for walk_path in walk_paths(recursive, depth, &path) {
    PATHS.write().unwrap().push(walk_path);
  }

  if matches.is_present("thumbnail") {
    if which("ffmpeg").is_ok() && which("ffprobe").is_ok() {
      transcode();
    } else {
      error!("\"ffmpeg\" and \"ffprobe\" need to be installed for transcode support.");
    }
  } else if matches.is_present("delete") {
    for path in PATHS
      .read()
      .unwrap()
      .iter()
      .filter(|p| path_is_thumbnail(p))
    {
      remove_file(path)?;
    }
  }

  // Watch for changes and update paths.
  watch()?;

  HttpServer::new(move || {
    actix_web::App::new()
      .wrap(Cors::default())
      .service(fs::Files::new("/static", "static").index_file("index.html"))
      .service(fs::Files::new("/media", &path).show_files_listing())
      .service(fs::Files::new("/thumbnails", &temp_dir).show_files_listing())
      .route("/", web::get().to(index))
      .route("/paths/", web::get().to(get_paths))
  })
  .workers(2)
  .bind(("0.0.0.0", 8288))?
  .run()
  .await?;

  Ok(())
}
