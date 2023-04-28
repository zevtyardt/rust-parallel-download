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
```bash
$ git clone https://github.com/zevtyardt/rust-parallel-download
$ cd rust-parallel-download/
$ cargo run --release
```

# Features
- [x] auto-detect file name and size
- [x] download files separately
- [x] realtime progress bar

# Any questions
Feel free to open a new issue 🥳
