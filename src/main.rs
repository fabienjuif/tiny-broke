use std::collections::HashMap;
use std::slice::Iter;
use std::time::SystemTime;
use zmq::{self, SocketType};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
struct Topic {
    name: String,
    workers: Vec<String>,
    next_worker_index: usize,
    clients: Vec<String>,
}

impl Topic {
    fn new(name: &str) -> Topic {
        Topic {
            name: name.to_string(),
            workers: vec![],
            next_worker_index: 0,
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
    sent: bool,
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
            sent: false,
        }
    }
}

struct Broker {
    clients: HashMap<String, Client>,
    topics: HashMap<String, Topic>,
    tasks: Vec<Task>,
}

impl Broker {
    fn new() -> Broker {
        Broker {
            clients: HashMap::new(),
            topics: HashMap::new(),
            tasks: Vec::new(),
        }
    }

    fn get_next_worker_name(&mut self, topic_name: &str) -> String {
        // TODO: round robin
        let topic = self.topics.get_mut(topic_name).unwrap();
        match topic.workers.get_mut(topic.next_worker_index) {
            Some(worker_name) => {
                topic.next_worker_index += 1;
                worker_name.clone()
            }
            None => {
                topic.next_worker_index = 1;
                topic.workers.get(0).unwrap().clone()
            }
        }
    }

    fn add_client(&mut self, is_worker: bool, identity: &str, response_topic: &str) {
        // add client
        let client = self
            .clients
            .entry(identity.to_string())
            .or_insert(Client::new(&identity, is_worker));
        client.topics.push(response_topic.to_string());

        // add topic
        let topic = self
            .topics
            .entry(response_topic.to_string())
            .or_insert(Topic::new(&response_topic));
        if is_worker {
            topic.workers.push(identity.to_string());
        } else {
            topic.clients.push(identity.to_string());
        }
    }

    fn send_task(&mut self, socket: &zmq::Socket, mut task: Task) -> Task {
        task.date = SystemTime::now();
        task.retry += 1;

        if task.retry >= 3 {
            // TODO: make it a const
            panic!("Max retry!");
        }

        // select a worker
        let worker_name = self.get_next_worker_name(&task.worker_topic);
        task.worker_name = Some(worker_name.clone());

        // send the task to the worker
        // if it doesn't works (worker is dead for instance), then we retry
        // the recursion is done if there is no worker anymore or if the retry is to damn high
        let sent = socket
            .send(&worker_name, zmq::SNDMORE | zmq::DONTWAIT)
            .and_then(|_| socket.send("", zmq::SNDMORE | zmq::DONTWAIT))
            .and_then(|_| socket.send(&task.payload, zmq::DONTWAIT));
        task.sent = sent.is_ok();

        task
    }

    fn send_response(&mut self, socket: &zmq::Socket, topic_name: &str, payload: &str) {
        let topic = self.topics.get(topic_name);
        if topic.is_none() {
            return;
        };
        let topic = topic.unwrap().clone();

        topic.clients.iter().for_each(|name| {
            socket
                .send(&name, zmq::SNDMORE | zmq::DONTWAIT)
                .and_then(|_| socket.send("", zmq::SNDMORE | zmq::DONTWAIT))
                .and_then(|_| socket.send(payload, zmq::DONTWAIT));

            let mut clients_to_remove = vec![];
            self.clients.entry(name.to_string()).and_modify(|client| {
                let position = client.topics.iter().position(|name| name == &topic.name);
                client.topics.remove(position.unwrap());
                if client.topics.is_empty() {
                    clients_to_remove.push(client.name.clone());
                }
            });

            clients_to_remove.iter().for_each(|name| {
                self.clients.remove(name);
            });
        });

        let topic = self.topics.get_mut(topic_name).unwrap();
        topic.clients.clear();

        if topic.workers.is_empty() {
            self.topics.remove(topic_name);
        }

        self.tasks.retain(|task| task.response_topic != topic_name);
    }

    fn remove_worker_from_topics(&mut self, worker: &Client) {
        worker.topics.iter().for_each(|topic| {
            self.topics.entry(topic.to_string()).and_modify(|topic| {
                let position = topic.workers.iter().position(|name| name == &worker.name);
                topic.workers.remove(position.unwrap());
            });
        });
    }

    fn remove_worker(&mut self, worker_name: &str) {
        let worker = self.clients.get(worker_name).unwrap();
        let worker = worker.clone(); // FIXME:
        self.remove_worker_from_topics(&worker);
        self.clients.remove(worker_name);
    }

    // TODO: should be accessible from a dedicated socket and only when the client ask for it
    //       it will speed up the overall process since it wouldn't have to use stdout for each task
    fn print_debug(&self) {
        let (workers, clients): (Vec<&Client>, Vec<&Client>) =
            self.clients.values().partition(|&client| client.is_worker);

        println!(
            "[{} workers; {} clients; {} topics; {} tasks]",
            &workers.len(),
            &clients.len(),
            &self.topics.len(),
            &self.tasks.len()
        );
    }
}

// TODO: don't use strings
fn main() {
    let context = zmq::Context::new();
    let socket = context.socket(SocketType::ROUTER).unwrap();
    socket.bind("tcp://127.0.0.1:3000").unwrap();

    // this to have error if a worker can't be reached
    socket.set_router_mandatory(true).unwrap();

    let mut message = zmq::Message::new();

    let mut broker = Broker::new();

    let mut index = 0;
    let mut identity = String::from("");
    let mut topic = String::from("");
    let mut response_topic = String::from("");
    let mut payload = String::from("");

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

        if message.get_more() {
            index += 1;
        } else {
            index = 0;

            if topic.as_str() == "@@REGISTER" {
                broker.add_client(true, &identity, &response_topic);
            } else if response_topic.len() == 0 {
                // worker response
                // TODO: find an other way, because a client may want to trigger an async action without waiting for acknowledgment
                broker.send_response(&socket, &topic, &payload);
            } else {
                // client ask for something
                broker.add_client(false, &identity, &response_topic);
                let mut task = Task::new(&topic, &response_topic, &payload);
                loop {
                    task = broker.send_task(&socket, task);
                    if !task.sent {
                        let worker_name = task.worker_name.as_ref().unwrap();
                        broker.remove_worker(&worker_name);
                    } else {
                        break;
                    }
                }
                broker.tasks.push(task);
            }

            broker.print_debug();
        }
    }
}
