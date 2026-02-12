use socket2::{Socket, Domain, Type, Protocol};

fn main() {
    match Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::UDP)) {
        Ok(_) => println!("RAW socket creation SUCCESS"),
        Err(e) => println!("RAW socket creation FAILED: {}", e),
    }
}
