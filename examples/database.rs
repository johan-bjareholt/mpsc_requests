use std::thread;
use std::collections::HashMap;
use mpsc_requests::channel;
#[derive(Debug)]
enum Errors {
    NoSuchPerson,
}
enum Commands {
    CreateUser(String, u64),
    GetUser(String)
}
#[derive(Debug)]
enum Responses {
    Success(),
    GotUser(u64)
}

fn main() {
    let (responder, requester) = channel::<Commands, Result<Responses, Errors>>();
    thread::spawn(move || {
        let mut age_table : HashMap<String, u64> = HashMap::new();
        responder.poll_loop(|mut request| {
            request.respond(match request.body() {
                Commands::CreateUser(user, age) => {
                    age_table.insert(user.to_string(), *age);
                    Ok(Responses::Success())
                },
                Commands::GetUser(user) => {
                    match age_table.get(user) {
                        Some(age) => Ok(Responses::GotUser(age.clone())),
                        None => Err(Errors::NoSuchPerson)
                    }
                }
            });
        });
    });

    // Create user
    let username = String::from("George");
    requester.request(Commands::CreateUser(username.clone(), 64)).unwrap();
    // Fetch user and verify data
    let command = Commands::GetUser(username.clone());
    match requester.request(command).unwrap() {
        Responses::GotUser(age) => assert_eq!(age, 64),
        _ => panic!("Wrong age")
    }
    // Try to fetch non-existing user
    let username = String::from("David");
    let command = Commands::GetUser(username);
    let result = requester.request(command);
    assert!(result.is_err());
}
