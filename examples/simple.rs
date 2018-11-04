use std::thread;
use mpsc_requests::channel;

fn main() {
    type RequestType = String;
    type ResponseType = String;
    let (responder, requester) = channel::<RequestType, ResponseType>();
    thread::spawn(move || {
        responder.poll_loop(|mut req| req.respond(req.body().clone()));
    });
    let msg = String::from("Hello");
    let res = requester.request(msg.clone());
    assert_eq!(res, msg);
}
