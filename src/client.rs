use crate::format::{Message, Type, VariableHeaderFactory, create_message, parse_message};
use crate::net::connect;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Instant;

pub struct Zabamqtt {
    identifiers: HashSet<u16>,
    header_factory: VariableHeaderFactory,
    client: Option<TcpStream>,
    keepAlive: u16,
    timer: Instant,
}

impl Zabamqtt {
    pub fn new() -> Self {
        Self {
            identifiers: HashSet::new(),
            header_factory: VariableHeaderFactory {},
            client: None,
            keepAlive: 0,
            timer: Instant::now(),
        }
    }

    pub fn connect_client(
        &mut self,
        ip: &str,
        client_id: &str,
        user: &str,
        pw: &str,
        will: u8,
        keep_alive: u16,
    ) -> Result<(), std::io::Error> {
        let stream = connect(ip)?;

        stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;

        self.client = Some(stream);
        self.keepAlive = keep_alive;
        self.reset_timer();

        let msg = self.create_msg(Type::CONNECT(client_id, user, pw, will, keep_alive));
        self.send(msg)?;

        let _ = self.receive()?;

        Ok(())
    }
    pub fn create_msg(&self, msg_type: Type) -> Message {
        create_message(msg_type, &self.header_factory)
    }

    pub fn receive(&mut self) -> Result<Option<Message>, std::io::Error> {
        let stream = match self.client.as_mut() {
            Some(s) => s,
            None => return Ok(None),
        };

        let mut buf = [0u8; 4096];
        match stream.read(&mut buf) {
            Ok(0) => {
                println!("Connection closed by broker");
                self.client = None;
                Ok(None)
            }
            Ok(n) => {
                self.reset_timer();
                let msg = parse_message(&buf[..n]); // Slice to received bytes only
                Ok(Some(msg))
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // Timeout elapsed without data, return Ok(None) to let caller loop
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }
    pub fn send(&mut self, msg: Message) -> Result<(), std::io::Error> {
        if let Some(ref mut stream) = self.client {
            let bytes = msg.to_bytes();
            println!("Sending {:?} bytes: {:02X?}", bytes.len(), bytes);
            stream.write_all(&bytes)?;
            stream.flush()?;
        }
        self.reset_timer();
        Ok(())
    }

    pub fn should_ping(&self) -> bool {
        self.client.is_some()
            && self.keepAlive > 0
            && self.timer.elapsed().as_secs() >= (self.keepAlive / 2) as u64
    }

    fn reset_timer(&mut self) {
        self.timer = Instant::now();
    }
}
