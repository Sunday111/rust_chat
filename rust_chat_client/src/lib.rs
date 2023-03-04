use std::io::{self, BufRead, Write};

pub mod application;
pub mod command;
pub mod client;

pub fn run_app() {
    let stdin = io::stdin();

    let username = {
        print!("Enter nickname: ");
        io::stdout().flush().unwrap();
        let mut username = String::new();
        stdin.lock().read_line(&mut username).unwrap();
        username.trim_end().to_string()
    };

    let mut stream = std::net::TcpStream::connect("127.0.0.1:8787").unwrap();
    stream
        .set_nonblocking(true)
        .expect("Failed to make tcp stream non-blocking");

    let mut receiver = rust_chat::PacketReceiver::new();
    let sender = std::cell::RefCell::new(rust_chat::PacketSender::new());

    let enqueue_message = |x: &str| {
        println!("Sedning this: {}", x);
        let mut data = Vec::new();
        data.extend_from_slice(&x.as_bytes());
        sender.borrow_mut().add_to_send_queue(data);
    };

    enqueue_message(&format!("{{ \"username\": \"{username}\" }}"));

    loop {
        sender.borrow_mut().advance(&mut stream).expect("Failed to advance");
        receiver.advance(&mut stream).expect("Failed to advance");

        while let Some(data) = receiver.pop_packet() {
            let message = String::from_utf8_lossy(&data);
            println!("{message}");
        }

        {
            let mut current = String::new();
            stdin.lock().read_line(&mut current).unwrap();
            if current.trim().len() > 0 {
                enqueue_message(current.trim());
            }
        }
    }
}
