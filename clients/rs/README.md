# tiny-broke-client (rust)

Rust client for [tiny-broke](https://github.com/fabienjuif/tiny-broke).

## Service
```rust
use tiny_broke_client::Broke;

fn main() {
  // you connect to the broker by giving a name, the broker uri, and "true" (meaning this is a worker)
  // at this time the Rust client can't be a client
  let mut broke = Broke::new("service-users", "tcp://localhost:3000", true);

  // then you register a closure to a message type
  // here, everytime "USER>GET_TOKEN" is send by the broker, we print "Hey you!"
  // the given message is a string (JSON), you have to parse it by yourself
  // you have to return a string, you have to serialize it by yourself, it will be added to the response payload
  broke.register(
    "USER>GET_TOKEN",
    &|message| {
      println!("Hey you! {}", message);

      return String::from("reponse")
    }
  );

  // then you have to listen to new events sent by the broker
  broke.run();
}

```

## Client
There is no client implementation at the moment.
