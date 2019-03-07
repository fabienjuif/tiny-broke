# tiny-broke
This RPC like application, the client ask for something, then register to a topic, waiting for response.

The worker register to some topics, and send back an event when its work is done, so the client can complete its own task.

You can find exemple usage here, [in the JavaScript client](https://github.com/fabienjuif/tiny-broke/blob/master/clients/js/README.md)

## Run tiny-broke
- `docker run -p 3000:3000 fabienjuif/tiny-broke`

## Configuration

You have to use environment variables to configure tiny-broke:
- `TASK_TIMEOUT`: **seconds** to wait for a worker response one we send the task to it. If the broker does not respond in time we drop the task
  * default value is `60` **seconds**

## Features
- Only one port to open
- RPC like communication, based on events
- Retry when no worker is available
- Heartbeating
- Task timeout
- Load balancing (round-robin)

## Roadmap
- Docker FROM scratch
- Handle CTRL+C
- Dedicated socket to retrieve stats
- UI to see those stats
- Retry timeout tasks (these are tasks that are send to a worker)
- Break the SPOF (by allowing multiple tiny-broke to speak together?)
- Client should be able to send a task and never wait a response (no returns type)

## Not in near future
- Persisting tasks (disk, db, whatever)
- SSL support (?)
- Authentication (?)
