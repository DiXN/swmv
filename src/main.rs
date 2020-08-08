#[macro_use]
extern crate log;

use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use clap::{value_t, App, Arg};
use directories::UserDirs;
use once_cell::sync::OnceCell;
use regex::Regex;
use walkdir::{DirEntry, WalkDir};

use actix::prelude::*;
use actix::{Actor, StreamHandler};
use actix_files as fs;
use actix_web::{web, Error, HttpRequest, HttpResponse, HttpServer, Responder, http};
use actix_web_actors::ws;
use actix_cors::Cors;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

static ARGS: OnceCell<Args> = OnceCell::new();

#[derive(Debug)]
struct Args {
  pub recursive: bool,
  pub path: PathBuf,
  pub depth: Option<usize>,
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

struct WebSocket {
  hb: Instant,
}

impl Actor for WebSocket {
  type Context = ws::WebsocketContext<Self>;

  /// Method is called on actor start. We start the heartbeat process here.
  fn started(&mut self, ctx: &mut Self::Context) {
    self.hb(ctx);
  }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocket {
  fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
    match msg {
      Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
      Ok(ws::Message::Text(text)) => ctx.text(text),
      Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
      _ => (),
    }
  }
}

async fn get_paths() -> impl Responder {
  let args = ARGS.get().unwrap();

  let walker = if args.recursive {
    WalkDir::new(&args.path)
  } else {
    WalkDir::new(&args.path).max_depth(1)
  };

  let walker = if let Some(depth) = args.depth {
    WalkDir::new(&args.path).max_depth(depth)
  } else {
    walker
  };

  web::Json(
    walker
      .into_iter()
      .filter_map(Result::ok)
      .filter(|e| validate_files(e))
      .map(|e| PathBuf::from(e.path().strip_prefix(&args.path).unwrap()))
      .collect::<Vec<_>>()
  )
}

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
  let resp = ws::start(WebSocket::new(), &req, stream);
  println!("{:?}", resp);
  resp
}

impl WebSocket {
  fn new() -> Self {
    Self { hb: Instant::now() }
  }

  fn hb(&self, ctx: &mut <Self as Actor>::Context) {
    ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
      if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
        info!("Websocket Client heartbeat failed, disconnecting!");

        ctx.stop();

        return;
      }

      ctx.ping(b"");
    });
  }
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

  let args = Args {
    recursive: matches.is_present("recursive"),
    path: path.clone(),
    depth: if matches.is_present("depth") {
      Some(value_t!(matches.value_of("depth"), usize).unwrap_or_else(|e| e.exit()))
    } else {
      None
    },
  };

  ARGS.set(args).unwrap();

  HttpServer::new(move || {
    actix_web::App::new()
      .wrap(
        Cors::new()
          .allowed_origin("index.html")
          .allowed_methods(vec!["GET"])
          .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
          .allowed_header(http::header::CONTENT_TYPE)
          .max_age(3600)
          .finish())
      .service(fs::Files::new("/static", "static").show_files_listing())
      .service(fs::Files::new("/media", &path).show_files_listing())
      .route("/ws/", web::get().to(index))
      .route("/paths/", web::get().to(get_paths))
  })
  .bind("127.0.0.1:8288")?
  .run()
  .await?;

  Ok(())
}
