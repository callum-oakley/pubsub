extern crate futures;
extern crate hyper;

use futures::sync::mpsc::{channel, Receiver, Sender};
use futures::{future, Stream};
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct Bus {
    sender: Mutex<Option<Sender<String>>>,
}

impl Bus {
    fn new() -> Self {
        Bus {
            sender: Mutex::new(None),
        }
    }

    fn subscribe(&self) -> Receiver<String> {
        let (s, r) = channel(0);
        *self.sender.lock().unwrap() = Some(s);
        r
    }

    fn send(&self, message: String) {
        if let Some(mut s) = self.sender.lock().unwrap().clone() {
            s.try_send(message).unwrap();
        }
    }
}

fn handle(req: Request<Body>, bus: Arc<Bus>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            *response.body_mut() = Body::wrap_stream(
                bus.subscribe()
                    .map_err(|_| Error::new(ErrorKind::Other, "oh no!")),
            )
        }
        (&Method::POST, "/") => {
            let response = req.into_body().concat2().map(move |data| {
                bus.send(String::from_utf8_lossy(&data.to_vec()).to_string());
                *response.status_mut() = StatusCode::CREATED;
                response
            });
            return Box::new(response);
        }
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Box::new(future::ok(response))
}

fn main() {
    let addr = ([127, 0, 0, 1], 3000).into();

    let bus = Arc::new(Bus::new());

    let server = Server::bind(&addr)
        .serve(move || {
            // TODO there must be a nicer way of doing this
            let bus = Arc::clone(&bus);
            service_fn(move |req| handle(req, Arc::clone(&bus)))
        })
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);

    rt::run(server);
}
