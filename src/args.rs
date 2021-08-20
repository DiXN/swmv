use clap::{App, Arg};

// Get command line arguments.
pub fn get_args<'a, 'b>() -> App<'a, 'b> {
  App::new("swmv")
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
    .arg(
      Arg::with_name("cuda")
        .short("c")
        .long("cuda")
        .help("Transcodes thumbnails with cuda.")
        .required(false)
        .takes_value(false),
    )
}
