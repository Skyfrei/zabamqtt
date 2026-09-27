use crate::format::{Message, Type, VariableHeaderFactory, create_message, receive_message};
use crate::net::connect;
use std::collections::HashSet;
use std::io::Write;
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
        let mut stream = connect(ip)?;
        let msg = self.create_msg(Type::CONNECT("", user, pw, will));
        self.send(msg);
        self.client = Some(stream.try_clone()?);
        Ok(stream)
    }

    pub fn create_msg(&self, msg_type: Type) -> Message {
        create_message(msg_type, &self.header_factory)
    }

    pub fn receive(&self) {
        receive_message(&self.header_factory);
    }

    pub fn send(&mut self, msg: Message) {
        if let Some(stream) = self.client.as_mut() {
            let bytes = msg.to_bytes();
            stream.write_all(&bytes);
        }
    }

    pub fn decode(&self, packet: &[u8]) {}
}
