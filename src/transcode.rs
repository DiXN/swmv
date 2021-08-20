use crate::path_is_thumbnail;
use crate::{ARGS, PATHS};

use std::path::{Path, PathBuf};
use std::process::Command;
use std::str;

use anyhow::Result;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct MetaData {
  pub width: usize,
  pub height: usize,
  pub duration: String,
  pub bit_rate: String,
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

pub fn transcode() {
  std::thread::spawn(move || {
    let g_args = ARGS.get().unwrap();

    let threads = num_cpus::get() / 4;
    let threads = if threads > 0 { threads } else { 1 };

    PATHS
      .read()
      .unwrap()
      .par_iter()
      .filter(|p| !path_is_thumbnail(p))
      .for_each(move |path: &PathBuf| {
        if path.extension().unwrap() == "mp4" {
          let meta_data = read_metadata(&path).unwrap_or(MetaData {
            width: 320,
            height: 240,
            duration: "15.0".to_owned(),
            bit_rate: "1010000".to_owned(),
          });

          let parse_duration = |duration: &str| -> usize {
            let mut splitted_duration = duration.split('.');
            let number = splitted_duration.next().unwrap();
            number.parse::<usize>().unwrap()
          };

          println!("Transcoding \"{}\"", path.display());

          let mut args = [
            if g_args.cuda {
              "-hwaccel".to_owned()
            } else {
              "-threads".to_owned()
            },
            if g_args.cuda {
              "cuda".to_owned()
            } else {
              format!("{}", threads)
            },
            "-i".to_owned(),
            path.display().to_string(),
            "-c:a".to_owned(),
            "copy".to_owned(),
            "-c:v".to_owned(),
            if g_args.cuda {
              "h264_nvenc".to_owned()
            } else {
              "libx264".to_owned()
            },
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
      });
  });
}
