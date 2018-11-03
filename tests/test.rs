#[cfg(test)]
mod tests {
    extern crate mpsc_requests;

    use std::thread;

    use mpsc_requests::{channel, Requester};

    // Tests responding with same data as was sent
    #[test]
    fn test_echo() {
        let (c, p) = channel::<String, String>();
        thread::spawn(move || {
            c.poll_loop(|req| req);
        });
        let msg = String::from("test");
        let result = p.send_req(msg.clone());
        assert_eq!(result, msg);
    }

    // Tests different types of the Request and Reponse types
    #[test]
    fn test_wordcount() {
        let (responder, requester) = channel::<String, usize>();
        thread::spawn(move || {
            responder.poll_loop(|req| req.len());
        });
        let msg = String::from("test");
        let result = requester.send_req(msg.clone());
        assert_eq!(result, 4);

        let msg = String::from("hello hello 123 123");
        let result = requester.send_req(msg.clone());
        assert_eq!(result, 19);
    }

    // Example of returning Result from request
    #[test]
    fn test_result() {
        #[derive(Debug)]
        struct InvalidStringError;
        let (responder, requester) = channel::<String, Result<(), InvalidStringError>>();
        thread::spawn(move || {
            responder.poll_loop(|req| {
                if req.starts_with("http://") {
                    Ok(())
                } else {
                    Err(InvalidStringError)
                }
            });
        });
        let msg = String::from("http://test.com");
        let result = requester.send_req(msg);
        result.unwrap();

        let msg = String::from("invalid string");
        let result = requester.send_req(msg);
        assert!(result.is_err());
    }

    // Test multiple requesters on multiple threads with unique requests
    #[test]
    fn test_multiple_requesters() {
        use std::sync::atomic::AtomicUsize;
        use std::sync::atomic::Ordering;
        use std::sync::Arc;

        const N_THREADS : usize = 10;
        const N_REQUESTS_PER_THREAD : i64 = 1000;

        let (responder, requester) = channel::<String, String>();
        thread::spawn(move || {
            responder.poll_loop(|req| req);
        });

        fn request_fn(requester: Requester<String, String>, ti: usize, atomic_counter: Arc<AtomicUsize>) {
            atomic_counter.fetch_add(1, Ordering::Acquire);
            thread::park();
            for i in 0..N_REQUESTS_PER_THREAD {
                let msg = format!("message from thread {} iteration {}", ti, i);
                let result = requester.send_req(msg.clone());
                assert_eq!(result, msg);
            }
            atomic_counter.fetch_sub(1, Ordering::Acquire);
        }

        let mut threads = vec![];
        let atomic_counter = Arc::new(AtomicUsize::new(0));
        for i in 0..N_THREADS {
            let thread_p = requester.clone();
            let a = atomic_counter.clone();
            let t = thread::spawn(move || request_fn(thread_p, i, a));
            threads.push(t);
        }
        while atomic_counter.load(Ordering::Acquire) < N_THREADS {}
        for t in threads {
            t.thread().unpark();
        }
        while atomic_counter.load(Ordering::Acquire) > 0 {}
    }
}
