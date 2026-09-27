use crate::format::{Type, VariableHeaderFactory, create_message, receive_message};
use std::collections::HashSet;

pub struct Zabaqtt {
    identifiers: HashSet<u16>,
    headerFactory: VariableHeaderFactory,
}

impl Zabaqtt {
    pub fn send(&self, msgType: Type) {
        create_message(msgType, &self.headerFactory);
    }

    pub fn receive(&self) {
        receive_message(&self.headerFactory);
    }
}
