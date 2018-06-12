extern crate futures;
extern crate hyper;

use futures::{future, Stream};
use hyper::rt::{self, Future};
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::sync::{Arc, Mutex};

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct Store {
    stored: Mutex<Option<String>>,
}

impl Store {
    fn new() -> Self {
        Self {
            stored: Mutex::new(None),
        }
    }

    fn get(&self) -> Option<String> {
        self.stored.lock().unwrap().clone()
    }

    fn set(&self, value: String) {
        *self.stored.lock().unwrap() = Some(value);
    }
}

fn handle(req: Request<Body>, store: Arc<Store>) -> BoxFut {
    let mut response = Response::new(Body::empty());

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            if let Some(stored) = store.get() {
                *response.body_mut() = Body::from(stored);
            } else {
                *response.body_mut() = Body::from("nothing here yet!");
            }
        }
        (&Method::POST, "/") => {
            let response = req.into_body().concat2().map(move |data| {
                store.set(String::from_utf8_lossy(&data.to_vec()).to_string());
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

    let store = Arc::new(Store::new());

    let server = Server::bind(&addr)
        .serve(move || {
            // TODO there must be a nicer way of doing this
            let store = Arc::clone(&store);
            service_fn(move |req| handle(req, Arc::clone(&store)))
        })
        .map_err(|e| eprintln!("server error: {}", e));

    println!("Listening on http://{}", addr);

    rt::run(server);
}
