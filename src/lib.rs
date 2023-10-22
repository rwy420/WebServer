use std::{thread, sync::{mpsc::{self, Receiver}, Arc, Mutex}, num::ParseIntError, fmt::format, path::Path, fs::{OpenOptions, File, remove_file, self}, time::Instant, io::{prelude, Seek, Write, BufReader, BufRead, self, Read, Split}};


pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let(sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {workers, sender}
    }

    pub fn execute<F>(&self, f:F) where F: FnOnce() + Send + 'static {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        Logger::log(format!("Terminating all workers"));

        for _  in &self.workers {
            self.sender.send(Message::Terminate).unwrap();
        }

        for worker in &mut self.workers {
            Logger::log(format!("Shutting down worker {}", worker.id));

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewJob(job )=> {
                    Logger::log(format!("Worker {} got a job; executing.", id));
                    job();
                }
                Message::Terminate => {
                    Logger::log(format!("Worker {} should terminate.", id));
                    break;
                }
            }
        });

        Worker {id, thread: Some((thread))}
    }
}

pub struct Logger;

impl Logger {
    pub fn log(msg: String) {
        let mut file = OpenOptions::new().append(true).
            create_new(!Path::new("log.txt").exists()).
            open("log.txt").expect("Unable to open file");
        file.write_all((msg + "\n").as_bytes()).expect("write failed");
    }
}

pub struct Config;

impl Config {
    pub fn loadConfig(name: String) -> String {
        let results = fs::read_to_string("config.txt");

        let contents = match results {
            Ok(message) => message,
            Err(error ) => String::from("Config file wasn't found!"),
        };

        let mut lines = contents.split("\n");

        let header = lines.next().expect("Unable to read file!");
        let mut port_line = lines.next().expect("Unable to read file!");
        let port_binding = port_line.replace("Port: ", "");
        port_line = &port_binding;
        let mut thread_pool_size_line = lines.next().expect("Unable to read file");
        let thread_binding = thread_pool_size_line.replace("ThreadPoolSize: ", "");
        thread_pool_size_line = &thread_binding;

        if name.eq("PORT") {
            return port_line.to_owned();
        }else if name.eq("THREAD_POOL_SIZE") {
            return thread_pool_size_line.to_owned();
        }else {
            return "Unable to read config file!".to_owned();
        }
    }
}