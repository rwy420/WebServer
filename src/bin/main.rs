use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::fs;
use std::path::Path;

use WebServer::Logger;
use WebServer::ThreadPool;
use WebServer::Config;

fn main() {
    if Path::new("log.txt").exists() {
        let _ = fs::remove_file(Path::new("log.txt"));
    }
    if (!Path::new("config.txt").exists()) {
        let mut file = fs::OpenOptions::new().append(true).create_new(true).
            open("config.txt").expect("Unable to open file");
        file.write_all("#Config for WebServer\n".as_bytes()).expect("write failed"); //Line 1
        file.write_all("Port: 7878\n".as_bytes()).expect("write failed"); //Line 2
        file.write_all("ThreadPoolSize: 5\n".as_bytes()).expect("write failed"); //Line 3
    }

    let listener = TcpListener::bind(format!("127.0.0.1:{}", Config::loadConfig("PORT"
        .to_owned()))).unwrap();
    let pool_size: usize = Config::loadConfig("THREAD_POOL_SIZE".to_owned()).parse::<usize>().unwrap();
    let pool = ThreadPool::new(pool_size);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {handle_connection(stream)});
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let index = b"GET / HTTP/1.1\r\n";
    let get = b"GET /";
    let mut bufferString = String::from_utf8(buffer.to_vec()).expect("Found invalid UTF-8");

    let(status_line, filename) = if buffer.starts_with(index) {
        Logger::log(format!("Response: 200 OK. File: {}", "index.html"));
        ("HTTP/1.1 200 OK", "www/index.html")
    }else if buffer.starts_with(get) {
        bufferString = bufferString.replace("GET /", "");
        bufferString = format!("www/{}", bufferString);
        let path: &str = bufferString.split_whitespace().next().unwrap();
        if Path::new(path).exists() {
            Logger::log(format!("Response: 200 OK. File: {}", path));
            ("HTTP/1.1 200 OK", path)
        }else {
            Logger::log(format!("Response: 404 NOT FOUND. File: {}", path));
            ("HTTP/1.1 404 NOT FOUND", "www/default/404.html")
        }
    }else {
        Logger::log(format!("Response: 400 BAD REQUEST."));
        ("HTTP/1.1 400 BAD REQUEST", "www/default/400.html")
    };
    
    let contents = fs::read_to_string(filename).unwrap();

    let response = format!("{}\r\nContent-Length: {}\r\n\r\n{}",status_line, contents.len(), contents);
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}