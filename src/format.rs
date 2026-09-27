pub enum Type<'a> {
    Reserved,
    CONNECT(&'a str, Option<&'a str>, Option<&'a str>, Option<u8>),
    CONNACK,
    PUBLISH,
    PUBACK,
    PUBREC,
    PUBREL,
    PUBCOMP,
    SUBSCRIBE,
    SUBACK,
    UNSUBSCRIBE,
    UNSUBACK,
    PINGREQ,
    PINGRESP,
    DISCONNECT,
    // other shit
}

struct VariableHeader {
    protocol_name_size: u16,
    protocol_name: String,
    protocol_version: u8,
    flags: u8, // check this
    keepAlive: u16,
}

impl VariableHeader {
    pub fn len(&self) -> usize {
        2 + self.protocol_name.len() + 1 + 1 + 2
    }
}

pub struct Message {
    fixed_header: u16,
    var_header: VariableHeader,
    payload: Vec<u8>,
}

fn createVarHeaderFlags(username: Option<&str>, pw: Option<&str>, will: Option<u8>) -> u8 {
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

fn create_var_header(f: u8) -> VariableHeader {
    VariableHeader {
        protocol_name: String::from("MQTT"),
        protocol_name_size: 4,
        protocol_version: 0x04,
        flags: f,
        keepAlive: 0xA,
    }
}

fn str_to_bytelen(s: &str) -> [u8; 2] {
    let len = s.len() as u16;
    len.to_be_bytes()
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

    let uuid = String::from("123");
    let mut header: u16 = 0x10;
    let flag = createVarHeaderFlags(username, pw, will);

    let var_header = create_var_header(flag);
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

pub fn createMessage(msgType: Type) -> Message {
    match msgType {
        Type::CONNECT(client_id, username, pw, will) => {
            return create_connect(client_id, username, pw, will);
        }
        Type::SUBSCRIBE => {}
        Type::SUBACK => {}
        _ => {
            panic!("Unsupported message type!");
        }
    }
}
