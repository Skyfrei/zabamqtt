use std::any::{Any, TypeId};
use std::io::Write;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum QoS {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum DUP {
    FirstOccasion,
    ReDelivery,
}
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum RETAIN {
    DontRetain,
    Retain,
}

pub enum Type<'a> {
    CONNECT(&'a str, &'a str, &'a str, u8, u16),
    CONNACK,
    PUBLISH(&'a str, u16, DUP, QoS, RETAIN, Vec<u8>),
    PUBACK(u16),
    PUBREC(u16),
    PUBREL(u16),
    PUBCOMP(u16),
    SUBSCRIBE(&'a [(&'a str, QoS)], u16),
    SUBACK(u16),
    UNSUBSCRIBE(&'a [&'a str], u16),
    UNSUBACK(u16),
    PINGREQ,
    PINGRESP,
    DISCONNECT,
}

#[derive(Debug)]
enum Response {
    CONNACK,
    PUBACK(u16),
    PUBREC(u16),
    PUBREL(u16),
    PUBCOMP(u16),
    SUBACK(u16),
    UNSUBACK(u16),
    PINGRESP,
}
#[derive(Debug)]
enum State {
    Pending(Response),
    Failed,
    Completed,
}

pub struct VariableHeaderFactory {}

impl VariableHeaderFactory {
    pub fn get_header(&self, msg_type: Type) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::new();
        match msg_type {
            Type::CONNECT(client_id, username, pw, will, alive_timer) => {
                header = self.get_connect_header(client_id, username, pw, will, alive_timer);
            }
            Type::PUBLISH(topic, id, _, qos, _, _) => {
                header = self.get_publish_header(topic, id, qos);
            }
            Type::PUBACK(id)
            | Type::PUBREC(id)
            | Type::PUBREL(id)
            | Type::PUBCOMP(id)
            | Type::SUBACK(id)
            | Type::UNSUBSCRIBE(_, id)
            | Type::UNSUBACK(id)
            | Type::SUBSCRIBE(_, id) => {
                header.extend_from_slice(&id.to_be_bytes());
            }
            _ => {}
        }

        header
    }

    fn get_connect_header(
        &self,
        client_id: &str,
        username: &str,
        pw: &str,
        will: u8,
        alive_timer: u16,
    ) -> Vec<u8> {
        let mut var_flags: u8 = 0xFE;

        if username == "" {
            var_flags &= 0x7F;
        }
        if pw == "" {
            var_flags &= 0xBF;
        }
        if will == 0 {
            var_flags &= 0xC3;
        } else {
            var_flags &= 0xC3;
            if (will & 0x04) > 0 {
                var_flags |= (will & 0x38) | 0x04;
            } else {
                var_flags &= 0xC3;
            }
        }

        let prot_name = "MQTT";
        let (prot_name_len, prot_name_bytes) = str_to_tuple(prot_name);
        let level: u8 = 0x04;
        let keep_alive: u16 = alive_timer;

        let mut header = Vec::with_capacity(10);
        header.extend_from_slice(&prot_name_len);
        header.extend_from_slice(prot_name_bytes);
        header.push(level);
        header.push(var_flags);
        header.extend_from_slice(&keep_alive.to_be_bytes());

        header
    }

    fn get_publish_header(&self, topic: &str, identifier: u16, qos: QoS) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::new();
        let topic_len = topic.len() as u16;
        header.extend_from_slice(&topic_len.to_be_bytes());
        header.extend_from_slice(topic.as_bytes());

        match qos {
            QoS::AtLeastOnce | QoS::ExactlyOnce => {
                header.extend_from_slice(&identifier.to_be_bytes());
            }
            QoS::AtMostOnce => {}
        }

        header
    }
}

#[derive(Debug)]
pub struct Message {
    fixed_header: [u8; 5],
    var_header: Vec<u8>,
    payload: Vec<u8>,
    state: State,
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload_len = encode_length(self.var_header.len() + self.payload.len());
        let msg_len = 1 + payload_len.1 + self.var_header.len() + self.payload.len();

        let mut bytes = Vec::with_capacity(msg_len);
        bytes.push(self.fixed_header[0]);
        bytes.extend_from_slice(&payload_len.0[0..payload_len.1]);
        bytes.extend_from_slice(&self.var_header);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn get_payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn get_fixed_header(&self) -> [u8; 5] {
        self.fixed_header
    }

    pub fn get_variable_header(&self) -> &[u8] {
        &self.var_header
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) {}
}

fn encode_length(mut x: usize) -> ([u8; 4], usize) {
    let mut bytes = [0u8; 4];
    let mut count = 0;

    loop {
        let mut encoded_byte = (x % 128) as u8;
        x /= 128;

        if x > 0 {
            encoded_byte |= 128;
        }
        bytes[count] = encoded_byte;
        count += 1;
        if x == 0 {
            break;
        }
    }

    (bytes, count)
}

fn decode_length(data: &[u8]) -> u32 {
    let mut multiplier: u32 = 1;
    let mut value = 0;
    let mut index = 0;
    let mut encoded_byte = 0;

    loop {
        encoded_byte = data[index];
        value += (encoded_byte as u32 & 127) * multiplier;
        multiplier *= 128;
        if multiplier > 128 * 128 * 128 {
            panic!("Something definitely wrong here chief");
        }

        if (encoded_byte & 128) == 0 {
            break;
        }
        index += 1;
    }

    value
}

fn decode_fixed_header(data: &[u8]) -> ([u8; 5], u8) {
    if data.len() < 2 {
        panic!("Fixed header is too short");
    }

    let mut header = [0u8; 5];
    header[0] = data[0];

    let length = decode_length(&data[1..]);

    let len_bytes: usize = match length {
        0..=127 => 1,
        128..=16_383 => 2,
        16_384..=2_097_151 => 3,
        _ => 4,
    };

    header[1..1 + len_bytes].copy_from_slice(&data[1..1 + len_bytes]);
    (header, len_bytes as u8)
}

fn decode_variable_header(data: &[u8], packet_byte: u8) -> (Vec<u8>, State) {
    let mut res: Vec<u8> = Vec::new();
    let flags = packet_byte & 0xF;
    let parse_var_header = |data: &[u8], bytes_to_read: u8| -> u16 {
        let count = bytes_to_read as usize;
        u16::from_be_bytes(data[..count].try_into().unwrap())
    };

    let state = match (packet_byte & 0xF0) >> 4 {
        2 => {
            res.extend_from_slice(&data[..2]);
            State::Pending(Response::CONNACK)
        }
        3 => {
            let topic_len = parse_var_header(data, 2) as usize;
            let qos = (flags & 0x06) >> 1;
            let var_len = 2 + topic_len + if qos > 0 { 2 } else { 0 };

            res.extend_from_slice(&data[..var_len]);

            match qos {
                1 => {
                    let id = parse_var_header(&data[2 + topic_len..], 2);
                    State::Pending(Response::PUBACK(id))
                }
                2 => {
                    let id = parse_var_header(&data[2 + topic_len..], 2);
                    State::Pending(Response::PUBREC(id))
                }
                _ => State::Completed,
            }
        }
        4 => {
            let id = parse_var_header(data, 2);
            State::Pending(Response::PUBACK(id))
        }
        5 => {
            let id = parse_var_header(data, 2);
            State::Pending(Response::PUBREC(id))
        }
        6 => {
            if flags != 2 {
                // close connection
                panic!("Wrong flags for pubrel")
            }
            let id = parse_var_header(data, 2);
            State::Pending(Response::PUBREL(id))
        }
        7 => {
            let id = parse_var_header(data, 2);
            State::Pending(Response::PUBCOMP(id))
        }
        9 => {
            res.extend_from_slice(&data[..2]);
            let id = parse_var_header(data, 2);
            State::Pending(Response::SUBACK(id))
        }
        11 => {
            res.extend_from_slice(&data[..2]);
            let id = parse_var_header(data, 2);
            State::Pending(Response::UNSUBACK(id))
        }
        13 => State::Pending(Response::PINGRESP),
        _ => State::Failed,
    };
    (res, state)
}

fn str_to_tuple(s: &str) -> ([u8; 2], &[u8]) {
    let len = s.len() as u16;
    (len.to_be_bytes(), s.as_bytes())
}

fn create_connect<'a>(
    client_id: &'a str,
    username: &'a str,
    pw: &'a str,
    will: u8,
    keep_alive: u16,
    factory: &VariableHeaderFactory,
) -> Message {
    if client_id.len() > 23 || client_id.is_empty() {
        panic!("Wrong length of client id");
    }

    let var_header = factory.get_header(Type::CONNECT(client_id, username, pw, will, keep_alive));
    let mut payload: Vec<u8> = Vec::new();

    let mut temp = str_to_tuple(client_id);
    payload.extend_from_slice(&temp.0);
    payload.extend_from_slice(temp.1);

    if !username.is_empty() {
        temp = str_to_tuple(username);
        payload.extend_from_slice(&temp.0);
        payload.extend_from_slice(temp.1);
    }

    if !pw.is_empty() {
        temp = str_to_tuple(pw);
        payload.extend_from_slice(&temp.0);
        payload.extend_from_slice(temp.1);
    }

    if will != 0 {
        payload.push(will);
    }

    Message {
        fixed_header: [0x10, 0, 0, 0, 0],
        var_header,
        payload,
        state: State::Completed,
    }
}

fn create_subscribe(subs: &[(&str, QoS)], id: u16, factory: &VariableHeaderFactory) -> Message {
    assert!(
        !subs.is_empty(),
        "SUBSCRIBE packet must contain at least one topic"
    );

    let var_header = factory.get_header(Type::SUBSCRIBE(subs, id));

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
        fixed_header: [0x82, 0, 0, 0, 0],
        var_header,
        payload,
        state: State::Completed,
    }
}

fn create_unsubscribe(topics: &[&str], id: u16, factory: &VariableHeaderFactory) -> Message {
    assert!(
        !topics.is_empty(),
        "UNSUBSCRIBE packet must contain at least one topic"
    );

    let var_header = factory.get_header(Type::UNSUBSCRIBE(topics, id));

    let payload: Vec<u8> = {
        let mut bytes: Vec<u8> = Vec::new();
        for topic in topics {
            let s = str_to_tuple(topic);
            bytes.extend_from_slice(&s.0);
            bytes.extend_from_slice(s.1);
        }
        bytes
    };

    Message {
        // Fixed header for UNSUBSCRIBE is 0xA2 (Type 10 with reserved bits 0010)
        fixed_header: [0xA2, 0, 0, 0, 0],
        var_header,
        payload,
        state: State::Completed,
    }
}

fn create_publish(
    topic: &str,
    identifier: u16,
    dup: DUP,
    qos: QoS,
    ret: RETAIN,
    factory: &VariableHeaderFactory,
    payload: Vec<u8>,
) -> Message {
    let var_header =
        factory.get_header(Type::PUBLISH(topic, identifier, dup, qos, ret, Vec::new()));

    // Spec [MQTT-3.3.1-2]: DUP flag MUST be set to 0 for all QoS 0 messages
    let dup_bit = match qos {
        QoS::AtMostOnce => 0,
        _ => dup as u8,
    };

    let qos_bits = qos as u8;
    let retain_bit = ret as u8;

    // Fixed header byte 0: Type 3 (PUBLISH) | DUP | QoS | RETAIN
    let byte_0: u8 = (3 << 4) | (dup_bit << 3) | (qos_bits << 1) | retain_bit;

    let state = match qos {
        QoS::AtLeastOnce => State::Pending(Response::PUBACK(identifier)),
        QoS::ExactlyOnce => State::Pending(Response::PUBREC(identifier)),
        QoS::AtMostOnce => State::Completed,
    };

    Message {
        fixed_header: [byte_0, 0, 0, 0, 0],
        var_header,
        payload,
        state,
    }
}

fn create_puback(identifier: u16) -> Message {
    Message {
        fixed_header: [0x40, 0x02, 0, 0, 0],
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
        state: State::Completed,
    }
}

fn create_pubrec(identifier: u16) -> Message {
    Message {
        fixed_header: [0x50, 0x02, 0, 0, 0],
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
        state: State::Completed,
    }
}

fn create_pubrel(identifier: u16) -> Message {
    Message {
        fixed_header: [0x62, 0x02, 0, 0, 0],
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
        state: State::Completed,
    }
}

fn create_pubcomp(identifier: u16) -> Message {
    Message {
        fixed_header: [0x70, 0x02, 0, 0, 0],
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
        state: State::Completed,
    }
}

fn create_disconnect() -> Message {
    Message {
        fixed_header: [0xE0, 0x00, 0, 0, 0],
        var_header: Vec::new(),
        payload: Vec::new(),
        state: State::Completed,
    }
}

fn create_pingreq() -> Message {
    Message {
        fixed_header: [0xC0, 0x00, 0, 0, 0],
        var_header: Vec::new(),
        payload: Vec::new(),
        state: State::Pending(Response::PINGRESP),
    }
}

pub fn create_message(msg_type: Type, factory: &VariableHeaderFactory) -> Message {
    match msg_type {
        Type::CONNECT(client_id, username, pw, will, keep_alive) => {
            return create_connect(client_id, username, pw, will, keep_alive, &factory);
        }
        Type::SUBSCRIBE(topics, id) => return create_subscribe(topics, id, &factory),
        Type::UNSUBSCRIBE(topics, id) => return create_unsubscribe(topics, id, &factory),
        Type::PUBLISH(topic, id, dup, qos, retain, payload) => {
            return create_publish(topic, id, dup, qos, retain, &factory, payload);
        }
        Type::DISCONNECT => return create_disconnect(),
        Type::PINGREQ => return create_pingreq(),
        _ => create_pingreq(),
    }
}

pub fn parse_message(data: &[u8]) -> Message {
    let (header, remaining_bytes) = decode_fixed_header(data);
    let first_byte = header[0];
    let offset = 1 + remaining_bytes as usize;
    let (var_header, state) = decode_variable_header(&data[offset..], first_byte);

    Message {
        fixed_header: header,
        var_header: var_header.clone(),
        payload: data[offset + var_header.len()..].to_vec(),
        state,
    }
}
