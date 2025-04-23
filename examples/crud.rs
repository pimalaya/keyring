#![cfg(feature = "std")]

use std::{
    env,
    io::{stdin, stdout, Write},
};

use keyring_flows::{handlers::std as handler_std, Entry, State};
use secrecy::ExposeSecret;

fn main() {
    env_logger::init();

    let service = match env::var("SERVICE") {
        Ok(service) => service,
        Err(_) => read_line("Keyring service?"),
    };

    let name = match env::var("NAME") {
        Ok(name) => name,
        Err(_) => read_line("Keyring entry name?"),
    };

    let password = match env::var("PASSWORD") {
        Ok(password) => password,
        Err(_) => read_line("Keyring entry password?"),
    };

    let entry = Entry::new(name).with_service(service);
    let mut state = State::new(entry);

    let res = handler_std::read(&mut state);
    println!("first read: {res:?}");

    println!("store new password");
    state.set_secret(password);
    handler_std::write(&mut state).unwrap();

    handler_std::read(&mut state).unwrap();
    let secret = state.take_secret().unwrap();
    let password = secret.expose_secret();
    println!("second read: {password:?}");

    println!("delete entry");
    handler_std::delete(&mut state).unwrap();
    println!("no third read possible");
    handler_std::read(&mut state).unwrap_err();
}

fn read_line(prompt: &str) -> String {
    print!("{prompt} ");
    stdout().flush().unwrap();

    let mut line = String::new();
    stdin().read_line(&mut line).unwrap();

    line.trim().to_owned()
}
