# SWMV - Simple Web Media Viewer

is a simple [Imgur](https://imgur.com/) inspired media viewer for viewing local media content in a browser.

## Usage

```
USAGE:
    swmv [FLAGS] [OPTIONS]

FLAGS:
        --delete       Deletes thumbnails.
    -h, --help         Prints help information
    -r, --recursive    Searches for images and videos recursively
    -t, --thumbnail    Creates short clips for the videos in the viewer. Requires FFMPEG and can be CPU intensive.
    -V, --version      Prints version information

OPTIONS:
    -d, --depth <depth>    Specifies the maximum depth of entries yield by the iterator
    -p, --path <path>      The path to search for images and videos
```

## Example

`swmv -r -t -p ~/Pictures`

Displays media in *~/Pictures* and all it's subfolders, as well as transcodes all video files for thumbnail previews.
