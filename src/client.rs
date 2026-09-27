use crate::format::{Message, Type, VariableHeaderFactory, create_message, receive_message};
use std::collections::HashSet;
use std::net::TcpStream;
use std::io;
use crate::net::{connect};

pub struct Zabaqtt {
    identifiers: HashSet<u16>,
    header_factory: VariableHeaderFactory,
    client: Option<TcpStream>,
}

impl Zabaqtt {

    pub fn new() -> Self{
        Self{
            identifiers: HashSet::new(),
            header_factory: VariableHeaderFactory{},
            client: None
        }
    }

    pub fn connect_client(&self, ip: &str, user: &str, pw: &str, will: u8) -> std::io::Result<()>{
        let res = connect(ip)?;
        self.client = Some(res);
        self.send(Type::CONNECT(_, user, pw, will));
        self.client.read();
        Ok(())
    }

    pub fn send(&self, msg_type: Type) {
        if 
        create_message(msg_type, &self.header_factory);
    }

    pub fn receive(&self) {
        receive_message(&self.header_factory);
    }

    pub fn decode(&self, packet: &[u8]) {}
}
