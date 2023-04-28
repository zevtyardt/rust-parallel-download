use std::{
    io::{stdin, stdout, SeekFrom, Write},
    path::Path,
};

use futures::stream;
use futures_util::StreamExt;
use indicatif::{HumanBytes, MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use tokio::{
    fs::{self, File},
    io::{self, AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
};

#[derive(Debug, Clone)]
struct Part {
    index: i32,
    offset: usize,
    size: usize,
}

impl Part {
    fn new(index: i32, offset: usize, size: usize) -> Self {
        Part {
            index,
            offset,
            size,
        }
    }

    fn to_string(&self) -> String {
        format!("bytes={}-{}", self.offset, self.offset + self.size - 1)
    }

    fn set_offset(&mut self, new_offset: usize) {
        self.size -= new_offset;
        self.offset += new_offset;
    }
}

#[derive(Debug)]
struct Downloader {
    url: String,
    max_conns: i32,
    client: Client,
    filename: String,
}

impl Downloader {
    fn new(url: String, max_conns: i32) -> Self {
        println!("[+] Building async client");
        let client = Client::builder().build().unwrap();
        Downloader {
            url,
            max_conns,
            client,
            filename: String::new(),
        }
    }

    async fn get_content_length(&mut self) -> usize {
        println!("[+] Requesting content-length from the server");

        let mut length = 0usize;
        let header = self.client.head(&self.url).send().await;
        if let Ok(resp) = header {
            let headers = resp.headers();
            self.filename = self.get_filename(resp.url().path().to_string());

            if let Some(content_length) = headers.get("content-length") {
                length = content_length.to_str().unwrap().parse::<usize>().unwrap();
            }
        }

        if length > 0 {
            println!("[+] File name: {}", self.filename);
            println!("[+] File size: {}", HumanBytes(length as u64));
        }
        length
    }

    fn get_filename(&self, url: String) -> String {
        let path = Path::new(&url);
        if let Some(filename) = path.file_name() {
            return filename.to_str().unwrap().to_string();
        } else {
            return self.get_filename(self.url.clone());
        }
    }

    fn get_parts(&self, length: usize) -> Vec<Part> {
        let mut parts = vec![];

        println!("[+] Split file into {} parts", self.max_conns);
        let mut size = length / self.max_conns as usize;
        for index in 0..self.max_conns {
            let offset = index as usize * size;
            if index == self.max_conns - 1 {
                size = length - size * index as usize;
            }

            parts.push(Part::new(index + 1, offset, size));
        }
        parts
    }

    async fn download(&self, part: &Part, bar: ProgressBar, mut file: File, is_single_part: bool) {
        let mut total_bytes = 0;
        if part.size > 0 {
            let request = self
                .client
                .get(&self.url)
                .header("Range", part.to_string())
                .send()
                .await;

            if let Ok(response) = request {
                let mut stream = response.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    if let Ok(bytes) = chunk {
                        file.write(&bytes).await.unwrap();
                        let len = bytes.len() as u64;
                        bar.inc(len);
                        total_bytes += len;

                        if is_single_part {
                            bar.set_length(total_bytes)
                        }
                    }
                }
            }
        } else {
            total_bytes = bar.length().unwrap();
        }
        bar.println(format!(
            "[+] Part-{} => downloaded {} of {}",
            part.index,
            HumanBytes(total_bytes),
            HumanBytes(bar.length().unwrap())
        ));
        bar.finish_and_clear();
    }

    fn is_downloadable(&self, length: &usize) -> bool {
        if *length == 0 {
            println!("[+] Remote file name has no length!");
            println!("[+] Failed writing received data to disk/application");
            return false;
        }
        true
    }

    async fn is_already_downloaded(&self, length: &usize) -> bool {
        let filename = &self.filename;
        let filepath = Path::new(filename);
        if filepath.exists() {
            if fs::metadata(filepath).await.unwrap().len() == *length as u64 {
                println!("[+] Aborting, file already downloaded!");
                return true;
            }
        }
        false
    }

    async fn check_metadata(&self) {
        let metaname = format!("parts/{}.metadata", &self.filename);
        let metapath = Path::new(&metaname);
        if metapath.exists() {
            let mut fp = File::open(&metapath).await.unwrap();
            let metadata = fp.read_i32().await.unwrap();
            if metadata != self.max_conns {
                println!("[+] Max number of parts has changed, restarting file download");

                for index in 0..self.max_conns {
                    let n = format!("parts/{}.part-{}", &self.filename, index + 1);
                    let part = Path::new(&n);
                    if part.exists() {
                        fs::remove_file(part).await.unwrap();
                    }
                }
            }
        }
        let mut fp = File::create(metapath).await.unwrap();
        fp.write_i32(self.max_conns).await.unwrap();
    }

    async fn start(&mut self) {
        let dir_parts = Path::new("parts/");
        if !dir_parts.is_dir() {
            if let Ok(_) = fs::create_dir(dir_parts).await {
                println!("[+] Parts folder created");
            }
        }

        let length = self.get_content_length().await;
        if !self.is_downloadable(&length) || self.is_already_downloaded(&length).await {
            return;
        }
        self.check_metadata().await;
        let mut parts = self.get_parts(length);

        let multi_progress = MultiProgress::new();
        let template = ProgressStyle::with_template(
            "[{spinner:.green}] {msg} => {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
        )
        .unwrap();

        let mut futures = vec![];
        let is_single_part = parts.len() == 1;

        for (index, part) in parts.iter_mut().enumerate() {
            let bar = multi_progress.add(ProgressBar::new(part.size as u64));
            bar.set_style(template.clone());
            bar.set_message(format!("Part-{}", index + 1));

            if index == 0 {
                bar.println("[+] Start downloading")
            }

            let file_part = format!("parts/{}.part-{}", &self.filename, index + 1);
            let mut file: File;
            if Path::new(&file_part).exists() {
                file = fs::OpenOptions::new()
                    .read(true)
                    .append(true)
                    .open(&file_part)
                    .await
                    .unwrap();
                let file_size = fs::metadata(&file_part).await.unwrap().len();
                file.seek(SeekFrom::Start(file_size)).await.unwrap();
                bar.set_position(file_size);
                part.set_offset(file_size as usize);

                if part.size > 0 {
                    bar.println(format!(
                        "[+] File {} already exists, resuming from {}",
                        &file_part,
                        HumanBytes(file_size)
                    ));
                }
            } else {
                file = File::create(file_part).await.unwrap()
            }

            let fut = self.download(part, bar, file, is_single_part);
            futures.push(fut);
        }

        let files = stream::iter(futures)
            .buffer_unordered(self.max_conns as usize)
            .collect::<Vec<()>>()
            .await;

        println!("[+] Merge {} parts into one file", files.len());
        let mut output = File::create(&self.filename).await.unwrap();
        for part in parts {
            let file_part = format!("parts/{}.part-{}", &self.filename, part.index);
            let path = Path::new(&file_part);
            let mut input = File::open(&path).await.unwrap();
            io::copy(&mut input, &mut output).await.unwrap();
            fs::remove_file(&path).await.unwrap();
        }
        fs::remove_file(format!("parts/{}.metadata", &self.filename))
            .await
            .unwrap();

        println!("[+] file downloaded {}", &self.filename);
    }
}

fn user_input(msg: &str) -> String {
    let mut buf = String::new();
    print!("{}", msg);
    stdout().flush().unwrap();
    stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

#[tokio::main()]
async fn main() {
    let url = user_input("[?] url: ");
    if let Ok(mut max_conns) = user_input("[?] max connections (limit 8): ").parse::<i32>() {
        if max_conns > 8 {
            max_conns = 8;
        }

        let mut downloader = Downloader::new(url, max_conns);
        downloader.start().await;
    } else {
        println!("[+] Enter a valid number")
    }
}
