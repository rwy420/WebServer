use std::net::TcpListener;
use std::net::TcpStream;
use std::io::prelude::*;
use std::fs;
use std::path::Path;
use std::vec;

use WebServer::Logger;
use WebServer::ThreadPool;
use WebServer::Config;
use WebServer::JsonConfig;
use WebServer::WebFile;
use walkdir::WalkDir;

fn main() {
    //Regenating new log.txt file
    if Path::new("log.txt").exists() {
        let _ = fs::remove_file(Path::new("log.txt"));
    }
    //Creating files.json file if it doesn't exist
    if (!Path::new("files.json").exists()) {
        let mut file = fs::OpenOptions::new().append(true).create_new(true).
            open("files.json").expect("Unable to open file"); 
    }
    //Get all (new) files in www dir
    let files = WalkDir::new("./www");
    let mut webFiles: Vec<WebFile> = Vec::new();
    //Looping through the files and adding to the list if the file wasn't registered yet
    for f in files {
        let path = f.unwrap().path().to_str().expect("Could not load files from direcoty")
            .replace("\\", "/");
        let mut registered = false;
        //Looping through registered files from last run to take them over if they were registered
        for registered_file in JsonConfig::loadFiles() {
            if path.eq(&registered_file.path) {
                webFiles.push(registered_file);
                registered = true;
            }
        }
        //Add the file if it wasn't registered previously
        if !registered {
            let web_file: WebFile = WebFile { path: (path).to_string(), public: true };
            webFiles.push(web_file);
        }
    }

    //Write all web files to files.json
    JsonConfig::writeFiles(webFiles);

    //Creating config.txt and writing the config if it doesn't exist
    if (!Path::new("config.txt").exists()) {
        let mut file = fs::OpenOptions::new().append(true).create_new(true).
            open("config.txt").expect("Unable to open file");
        file.write_all("#Config for WebServer\n".as_bytes()).expect("write failed"); //Line 1
        file.write_all("Port: 7878\n".as_bytes()).expect("write failed"); //Line 2
        file.write_all("ThreadPoolSize: 5\n".as_bytes()).expect("write failed"); //Line 3
    }
    //Creating server
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
    //Reading request
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let index = b"GET / HTTP/1.1\r\n";
    let get = b"GET /";
    let mut bufferString = String::from_utf8(buffer.to_vec()).expect("Found invalid UTF-8");
    //Returning the requeted file to the client
    let(status_line, filename) = if buffer.starts_with(index) {
        Logger::log(format!("Response: 200 OK. File: {}", "index.html"));
        ("HTTP/1.1 200 OK", "www/index.html")
    }else if buffer.starts_with(get) {
        bufferString = bufferString.replace("GET /", "");
        bufferString = format!("./www/{}", bufferString);
        //Get requested path
        let mut path: &str = bufferString.split_whitespace().next().unwrap();
        //Remove attributes
        if path.contains("?") {
            let index: usize = path.find("?").unwrap_or(path.len());
            path = path.split_at(index).0;
        }
        

        let mut web_file: WebFile = WebFile { path: ".".to_owned(), public: false };
        let mut exists = false;
        for file in JsonConfig::loadFiles() {
            if file.path.eq(path) {
                web_file = file;
                exists = true;
            }
        }
        if exists {
            if web_file.public {
                Logger::log(format!("Response: 200 OK. File: {}", path));
                ("HTTP/1.1 200 OK", path)
            }else {
                Logger::log(format!("Response: 403 FORBIDDEN. File: {}", path));
                ("HTTP/1.1 403 FORBIDDEN", "www/default/403.html")
            }
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