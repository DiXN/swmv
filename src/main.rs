#[macro_use]
extern crate log;
extern crate env_logger;

use std::fs::remove_file;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use anyhow::Result;
use clap::{value_t, App, Arg};
use directories::UserDirs;
use once_cell::sync::OnceCell;
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;
use walkdir::{DirEntry, WalkDir};
use which::which;

use actix_cors::Cors;
use actix_files as fs;
use actix_files::NamedFile;
use actix_web::{http, web, HttpServer, Responder};

static ARGS: OnceCell<Args> = OnceCell::new();

#[derive(Debug)]
struct Args {
  pub recursive: bool,
  pub path: PathBuf,
  pub depth: Option<usize>,
  pub paths: Vec<PathBuf>,
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
  let normailized_paths = args
    .paths
    .iter()
    .filter(|p| !path_is_thumbnail(p))
    .map(|p| PathBuf::from(p.strip_prefix(&args.path).unwrap()))
    .collect::<Vec<_>>();
  web::Json(normailized_paths)
}

async fn exists_file_on_server(file: web::Path<String>) -> impl Responder {
  let args = ARGS.get().unwrap();

  for p in args.paths.iter() {
    if p.ends_with(file.to_owned()) {
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
    let args = ARGS.get().unwrap();

    let threads = num_cpus::get() / 4;
    let threads = if threads > 0 { threads } else { 1 };

    for path in args.paths.iter().filter(|p| !path_is_thumbnail(p)) {
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
          .arg(path.parent().unwrap().join(&thumbnail))
          .output()
          .expect("Failed to transcode video");
      }
    }
  });
}

#[actix_rt::main]
async fn main() -> Result<()> {
  std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
  env_logger::init();

  let matches = App::new("Imgurx")
    .version("0.1.0")
    .author("Michael Kaltschmid <kaltschmidmichael@gmail.com>")
    .about("It's Imgur but actually not.")
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

  let path = matches.value_of("path").map(PathBuf::from).unwrap_or(
    UserDirs::new()
      .ok_or(io::Error::new(
        ErrorKind::NotFound,
        "Cannot get user directories.",
      ))?
      .picture_dir()
      .map(PathBuf::from)
      .expect("Canot find picture directory. Specify a valid path."),
  );

  let recursive = matches.is_present("recursive");

  let depth = if matches.is_present("depth") {
    Some(value_t!(matches.value_of("depth"), usize).unwrap_or_else(|e| e.exit()))
  } else {
    None
  };

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

  let paths = walker
    .into_iter()
    .filter_map(Result::ok)
    .filter(|e| validate_files(e))
    .map(|e| PathBuf::from(e.path()))
    .collect::<Vec<_>>();

  let args = Args {
    recursive: recursive,
    path: path.clone(),
    depth: depth,
    paths: paths,
  };

  ARGS.set(args).unwrap();

  if matches.is_present("thumbnail") {
    if which("ffmpeg").is_ok() && which("ffprobe").is_ok() {
      transcode();
    } else {
      error!("\"ffmpeg\" and \"ffprobe\" need to be installed for transcode support.");
    }
  } else if matches.is_present("delete") {
    let paths = &ARGS.get().unwrap().paths;
    for path in paths.iter().filter(|p| path_is_thumbnail(p)) {
      remove_file(path)?;
    }
  }

  HttpServer::new(move || {
    actix_web::App::new()
      .wrap(
        Cors::new()
          .allowed_origin("index.html")
          .allowed_methods(vec!["GET"])
          .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
          .allowed_header(http::header::CONTENT_TYPE)
          .max_age(3600)
          .finish(),
      )
      .service(fs::Files::new("/static", "static").index_file("index.html"))
      .service(fs::Files::new("/media", &path).show_files_listing())
      .route("/", web::get().to(index))
      .route("/paths/", web::get().to(get_paths))
      .route("/file/exists/{file}", web::get().to(exists_file_on_server))
  })
  .bind("127.0.0.1:8288")?
  .run()
  .await?;

  // ffmpeg -hwaccel cuvid -c:v h264_cuvid -resize 320x240 -i vid -ss 00:00:00 -t 00:00:20 -c:a copy -c:v h264_nv enc -b:v 3M output.mp4

  Ok(())
}
