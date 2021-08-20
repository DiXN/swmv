use crate::{ARGS, PATHS};

use std::path::PathBuf;

use anyhow::Result;

use walkdir::WalkDir;

use notify::event::{CreateKind, EventKind::*, ModifyKind, RenameMode};
use notify::{RecursiveMode, Watcher};

pub fn watch() -> Result<()> {
  let args = ARGS.get().unwrap();

  let add_paths = |paths: &Vec<PathBuf>| {
    for path in paths.iter() {
      if !PATHS.read().unwrap().contains(path) {
        PATHS.write().unwrap().push(path.clone());
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
                    && !PATHS.read().unwrap().contains(&entry.path().to_path_buf())
                  {
                    PATHS
                      .write()
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
                  .write()
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

  Ok(())
}
