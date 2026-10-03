mod client;
mod format;
use std::time::Duration;
mod net;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = client::Zabaqtt::new();

    // 1. Connect
    let _ = client.connect_client("127.0.0.1:1883", "Sky-gaming", "", "", 0, 10)?;
    println!("Connected to Mosquitto!");

    // 2. Subscribe to test topic
    let sub = client.create_msg(format::Type::SUBSCRIBE(
        &[("test/research", format::QoS::AtMostOnce)],
        1,
    ));
    client.send(sub)?;
    println!("Subscribed to test/research");

    // 3. Receive loop
    loop {
        if client.should_ping() {
            let ping = client.create_msg(format::Type::PINGREQ);
            client.send(ping)?;
        }

        match client.receive() {
            Ok(Some(msg)) => {
                let header_byte = msg.get_fixed_header()[0];
                let packet_type = header_byte >> 4;

                match packet_type {
                    3 => {
                        let payload = String::from_utf8_lossy(msg.get_payload());
                        println!("<- Incoming PUBLISH: {}", payload);
                    }
                    9 => println!("<- Received SUBACK"),
                    13 => println!("<- Received PINGRESP"),
                    _ => (),
                }
            }
            Ok(None) => {
                // No packet arrived on this cycle (timeout / no data yet)
            }
            Err(e) => {
                eprintln!("Socket error: {}", e);
            }
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
