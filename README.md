# rust-parallel-download
A simple program to download files in parallel, built with rust 🦀

# Screenshot
![](https://i.ibb.co/3SxBDmb/a.png)

# Prerequisites
- rust >= 1.68
- cargo >= 1.68

# Dependencies
```toml
futures = "0.3.28"
futures-util = "0.3.28"
clap = { version = "4.2.5", features = ["derive"] }
indicatif = { version = "0.17.3", features = ["rayon"] }
reqwest = { version = "0.11.16", features = ["stream"] }
tokio = { version = "1.28.0", features = ["rt-multi-thread", "macros", "fs"] }
```

# Run
Build release
```bash
$ git clone https://github.com/zevtyardt/rust-parallel-download
$ cd rust-parallel-download/
$ cargo run --release
```

Or install on your local system by running the following command
```bash
$ cargo install --git https://github.com/zevtyardt/rust-parallel-download
```

Then use `download_file` command to run it
```bash
$ download_file -h
Usage: download_file [OPTIONS] [URL]

Arguments:
  [URL]  Url of file to be downloaded

Options:
  -m, --max-connections <int>  Number of HTTP GET connections (2-16)
  -h, --help                   Print help
```

# Features
- [x] auto-detect file name and size
- [x] download files separately
- [x] realtime progress bar
- [x] command line argument

# Any questions
Feel free to open a new issue 🥳
