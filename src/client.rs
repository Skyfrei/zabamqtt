use crate::format::{
    Message, Type, VariableHeaderFactory, create_message, decode_length, parse_message,
};
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
        clean_session: bool,
    ) -> Result<(), std::io::Error> {
        let stream = connect(ip)?;

        stream.set_read_timeout(Some(std::time::Duration::from_millis(100)))?;

        self.client = Some(stream);
        self.keepAlive = keep_alive;
        self.reset_timer();

        let msg = self.create_msg(Type::CONNECT(
            client_id,
            user,
            pw,
            will,
            keep_alive,
            clean_session,
        ));
        self.send(msg)?;

        let _ = self.receive()?;

        Ok(())
    }
    pub fn create_msg(&self, msg_type: Type) -> Message {
        create_message(msg_type, &self.header_factory)
    }

    pub fn receive(&mut self) -> Result<Option<Message>, std::io::Error> {
        let mut header_byte = [0u8; 1];
        let mut len_bytes = Vec::with_capacity(4);
        let mut body = Vec::new();

        {
            let stream = match self.client.as_mut() {
                Some(s) => s,
                None => return Ok(None),
            };

            match stream.read(&mut header_byte) {
                Ok(0) => {
                    self.client = None;
                    return Ok(None);
                }
                Ok(_) => {}
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return Ok(None);
                }
                Err(e) => return Err(e),
            }

            let mut single_byte = [0u8; 1];
            loop {
                stream.read_exact(&mut single_byte)?;
                len_bytes.push(single_byte[0]);
                if (single_byte[0] & 128) == 0 {
                    break;
                }
            }

            let remaining_len = decode_length(&len_bytes) as usize;
            if remaining_len > 0 {
                body.resize(remaining_len, 0);
                stream.read_exact(&mut body)?;
            }
        }

        self.reset_timer();
        let mut packet = Vec::with_capacity(1 + len_bytes.len() + body.len());
        packet.push(header_byte[0]);
        packet.extend_from_slice(&len_bytes);
        packet.extend_from_slice(&body);

        let msg = parse_message(&packet);
        Ok(Some(msg))
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
