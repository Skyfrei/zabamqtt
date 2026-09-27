use crate::format::{Message, Type, VariableHeaderFactory, create_message, parse_message};
use crate::net::connect;
use std::collections::HashSet;
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct Zabaqtt {
    identifiers: HashSet<u16>,
    header_factory: VariableHeaderFactory,
    client: Option<TcpStream>,
}

impl Zabaqtt {
    pub fn new() -> Self {
        Self {
            identifiers: HashSet::new(),
            header_factory: VariableHeaderFactory {},
            client: None,
        }
    }

    pub fn connect_client(
        &mut self,
        ip: &str,
        user: &str,
        pw: &str,
        will: u8,
    ) -> Result<TcpStream, std::io::Error> {
        let stream = connect(ip)?;
        let msg = self.create_msg(Type::CONNECT("", user, pw, will));
        self.send(msg);
        self.client = Some(stream.try_clone()?);
        Ok(stream)
    }

    pub fn create_msg(&self, msg_type: Type) -> Message {
        create_message(msg_type, &self.header_factory)
    }

    pub fn receive(&mut self) -> Message {
        let mut buffer = [0u8; 128];

        let stream = self.client.as_mut().expect("Client stream not initialized");

        let bytes_read = stream
            .read(&mut buffer)
            .expect("Failed to read from stream");

        parse_message(&buffer[..bytes_read], &self.header_factory)
    }

    pub fn send(&mut self, msg: Message) {
        if let Some(stream) = self.client.as_mut() {
            let bytes = msg.to_bytes();
            stream.write_all(&bytes);
        }
    }

    pub fn decode(&self, packet: &[u8]) {}
}
