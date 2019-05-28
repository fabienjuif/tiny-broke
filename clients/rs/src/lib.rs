use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use uuid::Uuid;
use zmq;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Message {
    r#type: String,
    returns_type: String,
}

struct Registration {
    topic: String,
    callback: Rc<RefCell<Fn(String) -> String>>,
}

impl fmt::Debug for Registration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Implementation {{ topic: {} }}", self.topic)
    }
}

impl Registration {
    pub fn new(topic: &str, registration: &'static Fn(String) -> String) -> Registration {
        Registration {
            topic: topic.to_string(),
            callback: Rc::new(RefCell::new(registration.clone())),
        }
    }
}

pub struct Broke {
    socket: zmq::Socket,
    registrations: Vec<Registration>,
}

impl Broke {
    pub fn new(name: &str, uri: &str, worker: bool) -> Broke {
        let context = zmq::Context::new();
        let socket = context.socket(zmq::SocketType::DEALER).unwrap();
        let entity = format!(
            "{}-{}-{}",
            if worker { "worker" } else { "client" },
            name,
            Uuid::new_v4()
        );

        socket.set_identity(entity.as_bytes()).expect("Can't set zmq identity");
        socket.connect(uri).expect("Can't connect");

        Broke {
            socket,
            registrations: vec![],
        }
    }

    pub fn register(&mut self, topic: &str, callback: &'static Fn(String) -> String) {
        self.registrations.push(Registration::new(topic, callback));

        self.socket
            .send("@@REGISTER", zmq::SNDMORE | zmq::DONTWAIT)
            .and_then(|_| {
                self.socket
                    .send(&format!("@@ASKED>{}", topic), zmq::DONTWAIT)
            })
            .ok();
    }

    pub fn dispatch(&self, raw: &str) {
        let message: Message = serde_json::from_str(raw).unwrap();

        self.registrations
            .iter()
            .filter(|registration| registration.topic == message.r#type)
            .for_each(|registration| {
                let callback = registration.callback.borrow_mut();
                let payload = callback(raw.to_string());

                self.socket
                    .send(&message.returns_type, zmq::SNDMORE | zmq::DONTWAIT)
                    .and_then(|_| self.socket.send("", zmq::SNDMORE | zmq::DONTWAIT))
                    .and_then(|_| {
                        self.socket.send(
                            &format!(
                                "{{ \"type\": \"{}\", \"payload\": \"{}\" }}",
                                message.returns_type, payload
                            ),
                            zmq::DONTWAIT,
                        )
                    })
                    .ok();
            });
    }

    pub fn run(&self) {
        let mut zmq_message = zmq::Message::new();
        let mut index = 0;
        let mut identity = String::from("");
        let mut raw = String::from("");

        loop {
            self.socket.recv(&mut zmq_message, 0).unwrap();
            let part = zmq_message.as_str().unwrap().to_owned();

            match index {
                0 => identity = part,
                1 => raw = part,
                _ => panic!(format!("Unknown index for message: {}", index)),
            }

            if zmq_message.get_more() {
                index += 1;
            } else {
                index = 0;

                self.dispatch(&raw);
            }
        }
    }
}
