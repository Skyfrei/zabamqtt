use std::io::Write;

#[derive(Clone)]
pub enum QoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}
pub enum Type<'a> {
    Reserved,
    CONNECT(&'a str, Option<&'a str>, Option<&'a str>, Option<u8>),
    CONNACK,
    PUBLISH,
    PUBACK,
    PUBREC,
    PUBREL,
    PUBCOMP,
    SUBSCRIBE(&'a [(&'a str, QoS)]),
    SUBACK,
    UNSUBSCRIBE,
    UNSUBACK,
    PINGREQ,
    PINGRESP,
    DISCONNECT,
    // other shit
}

struct VariableHeader {
    packet_identifier: u16,
    protocol_name_size: u16,
    protocol_name: String,
    level: u8,
    flags: u8, // check this
    keepAlive: u16,
}

pub struct Message {
    fixed_header: u16, // 2nd byte standing for length of payload is not calculated properly yet
    var_header: VariableHeader,
    payload: Vec<u8>,
}

impl VariableHeader {
    pub fn len(&self) -> usize {
        if self.packet_identifier == 0 {
            return 0 + 2 + self.protocol_name.len() + 1 + 1 + 2;
        }
        2 + 2 + self.protocol_name.len() + 1 + 1 + 2
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        if self.packet_identifier != 0 {
            bytes.extend_from_slice(&self.packet_identifier.to_be_bytes());
        }
        bytes.extend_from_slice(&self.protocol_name_size.to_be_bytes());
        bytes.extend_from_slice(&self.protocol_name.as_bytes());
        bytes.push(self.level);
        bytes.push(self.flags);
        bytes.extend_from_slice(&self.keepAlive.to_be_bytes());
        bytes
    }
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        let var_bytes = self.var_header.to_bytes();
        let payload_length = self.payload.len();

        let mut bytes = Vec::with_capacity(2 + var_bytes.len() + payload_length);
        bytes.extend_from_slice(&self.fixed_header.to_be_bytes());
        bytes.extend_from_slice(&var_bytes);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) {}
}

fn create_var_header_flags(username: Option<&str>, pw: Option<&str>, will: Option<u8>) -> u8 {
    let mut var_flags: u8 = 0xFE;

    if username.is_none() {
        var_flags &= 0x7F; // Bit 7: User Name Flag off
    }
    if pw.is_none() {
        var_flags &= 0xBF; // Bit 6: Password Flag off
    }
    match will {
        Some(x) => {
            var_flags &= 0xC3;
            if (x & 0x04) > 0 {
                var_flags |= (x & 0x38) | 0x04;
            } else {
                var_flags &= 0xC3;
            }
        }
        None => {
            var_flags &= 0xC3;
        }
    }

    var_flags
}

fn create_var_header(ident: u16, f: u8) -> VariableHeader {
    VariableHeader {
        packet_identifier: ident,
        protocol_name: String::from("MQTT"),
        protocol_name_size: 4,
        level: 0x04,
        flags: f,
        keepAlive: 0xA,
    }
}

fn str_to_bytelen(s: &str) -> [u8; 2] {
    let len = s.len() as u16;
    len.to_be_bytes()
}

fn str_to_tuple(s: &str) -> ([u8; 2], &[u8]) {
    let len = s.len() as u16;
    (len.to_be_bytes(), s.as_bytes())
}

fn create_connect<'a>(
    clientId: &'a str,
    username: Option<&'a str>,
    pw: Option<&'a str>,
    will: Option<u8>,
) -> Message {
    if clientId.len() > 23 || clientId.len() < 1 {
        panic!("Wrong length of client id");
    }

    let mut header: u16 = 0x10;
    let flag = create_var_header_flags(username, pw, will);

    let var_header = create_var_header(0, flag);
    let mut payload: Vec<u8> = Vec::new();

    payload.extend_from_slice(&str_to_bytelen(clientId));
    payload.extend_from_slice(clientId.as_bytes());
    if let Some(user) = username {
        payload.extend_from_slice(&str_to_bytelen(user));
        payload.extend_from_slice(user.as_bytes());
    }
    if let Some(pw) = pw {
        payload.extend_from_slice(&str_to_bytelen(pw));
        payload.extend_from_slice(pw.as_bytes());
    }
    if let Some(w) = will {
        payload.push(w);
    }

    Message {
        fixed_header: header,
        var_header: var_header,
        payload: payload,
    }
}

fn create_subscribe(subs: &[(&str, QoS)]) -> Message {
    assert!(
        !subs.is_empty(),
        "SUBSCRIBE packet must contain at least one topic"
    );

    let header: u16 = 0x82;

    let varHeader = create_var_header(0xA, 0x0);
    let payload: Vec<u8> = {
        let mut bytes: Vec<u8> = Vec::new();
        for &(topic, ref qos) in subs {
            let s = str_to_tuple(topic);
            bytes.extend_from_slice(&s.0);
            bytes.extend_from_slice(s.1);
            bytes.push(qos.clone() as u8);
        }
        bytes
    };

    Message {
        fixed_header: header,
        var_header: varHeader,
        payload: payload,
    }
}

pub fn createMessage(msgType: Type) -> Message {
    match msgType {
        Type::CONNECT(client_id, username, pw, will) => {
            return create_connect(client_id, username, pw, will);
        }
        Type::SUBSCRIBE(topics) => return create_subscribe(topics),
        Type::SUBACK => {}
        _ => {
            panic!("Unsupported message type!");
        }
    }
}
