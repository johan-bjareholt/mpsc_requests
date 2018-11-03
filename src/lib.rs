//! mpsc_requests is a small library built on top of std::sync::mpsc but with
//! the addition of the consumer responding with a message to the producer.
//! Since the producer no longer only produces and the consumer no longer only consumes, the
//! producer is renamed to requester and the consumer is renamed to responder.
//!
//! mpsc_requests is small and lean by only building on top of the rust standard library
//!
//! A perfect use-case for this library is single-threaded databases which need
//! to be accessed from multiple threads (such as SQLite)
//!
//! # Echo example
//! ```rust,run
//! use std::thread;
//! use mpsc_requests::channel;
//!
//! type RequestType = String;
//! type ResponseType = String;
//! let (responder, requester) = channel::<RequestType, ResponseType>();
//! thread::spawn(move || {
//!     responder.poll_loop(|req| req);
//! });
//! let msg = String::from("Hello");
//! let res = requester.send_req(msg.clone());
//! assert_eq!(res, msg);
//! ```
//! # Database example
//! ```rust,run
//! use std::thread;
//! use std::collections::HashMap;
//! use mpsc_requests::channel;
//! #[derive(Debug)]
//! enum Errors {
//!     NoSuchPerson,
//! }
//! enum Commands {
//!     CreateUser(String, u64),
//!     GetUser(String)
//! }
//! #[derive(Debug)]
//! enum Responses {
//!     Success(),
//!     GotUser(u64)
//! }
//! let (responder, requester) = channel::<Commands, Result<Responses, Errors>>();
//! thread::spawn(move || {
//!     let mut age_table : HashMap<String, u64> = HashMap::new();
//!     loop {
//!         responder.poll(|request| {
//!             match request {
//!                 Commands::CreateUser(user, age) => {
//!                     age_table.insert(user, age);
//!                     Ok(Responses::Success())
//!                 },
//!                 Commands::GetUser(user) => {
//!                     match age_table.get(&user) {
//!                         Some(age) => Ok(Responses::GotUser(age.clone())),
//!                         None => Err(Errors::NoSuchPerson)
//!                     }
//!                 }
//!             }
//!         });
//!     }
//! });
//!
//! // Create user
//! let username = String::from("George");
//! requester.send_req(Commands::CreateUser(username.clone(), 64)).unwrap();
//! // Fetch user and verify data
//! let command = Commands::GetUser(username.clone());
//! match requester.send_req(command).unwrap() {
//!     Responses::GotUser(age) => assert_eq!(age, 64),
//!     _ => panic!("Wrong age")
//! }
//! // Try to fetch non-existing user
//! let username = String::from("David");
//! let command = Commands::GetUser(username);
//! let result = requester.send_req(command);
//! assert!(result.is_err());
//! ```

#![deny(missing_docs)]

use std::sync::mpsc;

/// Create a channel between one requester and one responder.
/// The requester can be cloned to be able to do requests to the same responder from multiple
/// threads.
pub fn channel<Req, Res>() -> (Responder<Req, Res>, Requester<Req, Res>) {
    let (request_sender, request_receiver) = mpsc::channel::<Request<Req, Res>>();
    let c = Responder::new(request_receiver);
    let p = Requester::new(request_sender);
    return (c, p)

}

#[derive(Debug)]
/// Errors which can occur when a responder handles a request
pub enum RequestError {
    /// Error occuring when channel from requester to sender is broken
    RecvError,
    /// Error occuring when channel from sender to requester is broken
    SendError
}
impl From<mpsc::RecvError> for RequestError {
    fn from(_err: mpsc::RecvError) -> RequestError {
        RequestError::RecvError
    }
}
impl<T> From<mpsc::SendError<T>> for RequestError {
    fn from(_err: mpsc::SendError<T>) -> RequestError {
        RequestError::SendError
    }
}

struct Request<Req, Res> {
    request: Req,
    response_sender: mpsc::Sender<Res>
}

/// A responder listens to requests of a specific type and responds back to the requester
pub struct Responder<Req, Res> {
    request_receiver: mpsc::Receiver<Request<Req, Res>>,
}

impl<Req, Res> Responder<Req, Res> {
    fn new(request_receiver: mpsc::Receiver<Request<Req, Res>>) -> Responder<Req, Res> {
        Responder {
            request_receiver: request_receiver,
        }
    }

    /// Poll is the consumption function in the Responder which takes a request, handles it and
    /// sends a response back to the Requester
    pub fn poll<F>(&self, f: F) -> Result<(), RequestError> where F: FnOnce(Req) -> Res {
        let request = self.request_receiver.recv()?;
        let response = f(request.request);
        Ok(request.response_sender.send(response)?)
    }

    /// A shorthand for running poll with a closure for as long as there are any Requesters alive
    pub fn poll_loop(&self, f: fn(Req) -> Res) {
        loop {
            match self.poll(f){
                Ok(_) => (),
                Err(e) => match e {
                    // No more send channels open, quitting
                    RequestError::RecvError => break,
                    RequestError::SendError => panic!("Request failed, send pipe was broken during request!")
                }
            }
        }
    }
}

/// Requester has a connection to a Responder which it can send requests to
#[derive(Clone)]
pub struct Requester<Req, Res> {
    request_sender: mpsc::Sender<Request<Req, Res>>,
}

impl<Req, Res> Requester<Req, Res> {
    fn new(request_sender: mpsc::Sender<Request<Req, Res>>) -> Requester<Req, Res> {
        Requester {
            request_sender: request_sender,
        }
    }

    /// Send request to the connected Responder
    pub fn send_req(&self, req: Req) -> Res {
        let (response_sender, response_receiver) = mpsc::channel::<Res>();
        let full_request = Request {
            request: req,
            response_sender: response_sender
        };
        self.request_sender.send(full_request).unwrap();
        response_receiver.recv().unwrap()
    }
}
