use crate::format::{Message, Type, VariableHeaderFactory, create_message, receive_message};
use std::collections::HashSet;

pub struct Zabaqtt {
    identifiers: HashSet<u16>,
    header_factory: VariableHeaderFactory,
}

impl Zabaqtt {
    pub fn send(&self, msg_type: Type) {
        create_message(msg_type, &self.header_factory);
    }

    pub fn receive(&self) {
        receive_message(&self.header_factory);
    }

    pub fn decode(&self, packet: &[u8]) {}
}
