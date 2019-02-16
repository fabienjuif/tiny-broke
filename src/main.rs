use zmq::{self, SocketType};
use std::time::SystemTime;
use std::slice::Iter;
use std::collections::HashMap;

#[derive(Debug)]
struct Client {
    name: String,
    is_worker: bool,
    topics: Vec<String>,
}

impl Client {
    fn new(name: &str, is_worker: bool) -> Client {
        Client {
            is_worker,
            name: name.to_string(),
            topics: vec![],
        }
    }
}

#[derive(Debug)]
struct Topic<'a> {
    name: String,
    workers: Vec<String>,
    workers_iterator: Option<Iter<'a, String>>,
    clients: Vec<String>,
}

impl<'a> Topic<'a> {
    fn new(name: &str) -> Topic<'a> {
        Topic {
            name: name.to_string(),
            workers: vec![],
            workers_iterator: None,
            clients: vec![],
        }
    }
}

#[derive(Debug)]
struct Task {
    worker_topic: String,
    worker_name: Option<String>,
    response_topic: String,
    retry: u8,
    payload: String,
    date: SystemTime,
}

impl Task {
    fn new(worker_topic: &str, response_topic: &str, payload: &str) -> Task {
        Task {
            worker_topic: worker_topic.to_string(),
            worker_name: None,
            response_topic: response_topic.to_string(),
            retry: 0,
            payload: payload.to_string(),
            date: SystemTime::now(),
        }
    }
}

// TODO: should be accessible from a dedicated socket and only when the client ask for it
//       it will speed up the overall process since it wouldn't have to use stdout for each task
fn print_debug(all_clients: &HashMap<String, Client>, topics: &HashMap<String, Topic>, tasks: &Vec<Task>) {
    let (workers, clients): (Vec<&Client>, Vec<&Client>) = all_clients
        .values()
        .partition(|&client| client.is_worker);

    println!("[{} workers; {} clients; {} topics; {} tasks]", &workers.len(), &clients.len(), &topics.len(), &tasks.len());
}

// TODO: don't use strings
fn main() {
    let context = zmq::Context::new();
    let socket = context.socket(SocketType::ROUTER).unwrap();
    socket.bind("tcp://127.0.0.1:3000").unwrap();

    let mut message = zmq::Message::new();

    let mut index = 0;
    let mut identity = String::from("");
    let mut topic = String::from("");
    let mut response_topic = String::from("");
    let mut payload = String::from("");

    // TODO: change collection
    let mut clients = HashMap::<String, Client>::new();
    let mut topics = HashMap::<String, Topic>::new();
    let mut tasks = vec![];

    loop {
        socket.recv(&mut message, 0).unwrap();
        let part = message.as_str().unwrap().to_owned();

        match index {
            0 => identity = part,
            1 => topic = part,
            2 => response_topic = part,
            3 => payload = part,
            _ => panic!(format!("Unknown index for message: {}", index)),
        }

        let mut add_client = |is_worker| {
            // add client
            let client = clients
                .entry(identity.clone())
                .or_insert(Client::new(&identity, is_worker));
            client.topics.push(response_topic.clone());

            // add topic
            let topic = topics
                .entry(response_topic.clone())
                .or_insert(Topic::new(&response_topic));
            if is_worker {
                topic.workers.push(identity.clone());
            } else {
                topic.clients.push(identity.clone());
            }
        };

        let get_next_worker_name = |topic_name: &str, topics: &HashMap<String, Topic>, clients: &HashMap<String, Client>| {
            // TODO: round robin
            let topic = topics.get(topic_name).unwrap();
            let worker = clients.get(topic.workers.get(0).unwrap()).unwrap();

            worker.name.clone()
        };

        let send_task = |mut task: Task, topics: &HashMap<String, Topic>, clients: &HashMap<String, Client>| {
            task.date = SystemTime::now();
            task.retry += 1;

            if task.retry >= 3 { // TODO: make it a const
                panic!("Max retry!");
            }

            // select a worker
            let worker_name = get_next_worker_name(&task.worker_topic, topics, clients);
            task.worker_name = Some(worker_name.clone());

            // send the task to the worker
            // if it doesn't works (worker is dead for instance), then we retry
            // the recursion is done if there is no worker anymore or if the retry is to damn high
            dbg!(&task);
            // socket.send_multipart(&vec![worker_name.as_bytes(), task.payload.as_bytes()], zmq::DONTWAIT);
            socket.send(&worker_name, zmq::SNDMORE & zmq::DONTWAIT).unwrap();
            socket.send("", zmq::SNDMORE & zmq::DONTWAIT).unwrap(); // TODO: this could be removed!
            socket.send(&task.payload, zmq::DONTWAIT).unwrap();
            // TODO: const sent = send(sock, [worker.name, '', task.payload])
            // TODO: if (!sent) return sendTask(sock, task)
        };

        if message.get_more() {
            index += 1;
        } else {
            index = 0;

            if topic.as_str() == "@@REGISTER" {
                add_client(true);
            } else {
                add_client(false);
                let task = Task::new(&topic, &response_topic, &payload);
                send_task(task, &topics, &clients);
                // TODO: add task to vec
            }
            // TODO: handle worker response

            print_debug(&clients, &topics, &tasks);
        }
    }
}
