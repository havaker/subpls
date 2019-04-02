# Subpls
Subpls is a simple console app that downloads subtitles from https://opensubtitles.org using their xml-rpc api.
# Usage
To download english subtitles for `test.mp4` use:
```
subpls test.mp4 -l eng
```
For more options check:
```
subpls --help
```

# Building
```
cargo build
```
If you don't have cargo installed, go to https://rustup.rs/.
