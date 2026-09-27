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
    Reserved,
    CONNECT(&'a str, &'a str, &'a str, u8),
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
    // other shit
}

pub struct VariableHeaderFactory {}

impl VariableHeaderFactory {
    pub fn get_header(&self, msg_type: Type) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::new();
        match msg_type {
            Type::Reserved => {}
            Type::CONNECT(client_id, username, pw, will) => {
                header = self.get_connect_header(client_id, username, pw, will);
            }
            Type::PUBLISH(topic, id, _, _, _, _) => {
                header = self.get_publish_header(topic, id);
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

    fn get_connect_header(&self, client_id: &str, username: &str, pw: &str, will: u8) -> Vec<u8> {
        let mut var_flags: u8 = 0xFE;

        if username == "" {
            var_flags &= 0x7F; // Bit 7: User Name Flag off
        }
        if pw == "" {
            var_flags &= 0xBF; // Bit 6: Password Flag off
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
        let level: u8 = 0x04; // MQTT 3.1.1
        let keep_alive: u16 = 0x000A;

        let mut header = Vec::with_capacity(10);
        header.extend_from_slice(&prot_name_len);
        header.extend_from_slice(prot_name_bytes);
        header.push(level);
        header.push(var_flags);
        header.extend_from_slice(&keep_alive.to_be_bytes());

        header
    }

    fn get_publish_header(&self, topic: &str, identifier: u16) -> Vec<u8> {
        let mut header: Vec<u8> = Vec::new();
        let topic_len = topic.len() as u16;
        header.extend_from_slice(&topic_len.to_be_bytes());
        header.extend_from_slice(topic.as_bytes());
        header.extend_from_slice(&identifier.to_be_bytes());
        header
    }
}

pub struct Message {
    fixed_header: u16, // 2nd byte standing for length of payload is not calculated properly yet
    var_header: Vec<u8>,
    payload: Vec<u8>,
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        let payload_length = self.payload.len();

        let mut bytes = Vec::with_capacity(2 + self.var_header.len() + payload_length);
        bytes.extend_from_slice(&self.fixed_header.to_be_bytes());
        bytes.extend_from_slice(&self.var_header);
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) {}
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
    factory: &VariableHeaderFactory,
) -> Message {
    if client_id.len() > 23 || client_id.len() < 1 {
        panic!("Wrong length of client id");
    }

    let header: u16 = 0x10;
    let var_header = factory.get_header(Type::CONNECT(client_id, username, pw, will));

    let mut payload: Vec<u8> = Vec::new();

    let mut temp = str_to_tuple(client_id);

    payload.extend_from_slice(&temp.0);
    payload.extend_from_slice(temp.1);
    if let Some(user) = username {
        temp = str_to_tuple(user);
        payload.extend_from_slice(&temp.0);
        payload.extend_from_slice(temp.1);
    }
    if let Some(pw) = pw {
        temp = str_to_tuple(pw);
        payload.extend_from_slice(&temp.0);
        payload.extend_from_slice(temp.1);
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

fn create_subscribe(subs: &[(&str, QoS)], id: u16, factory: &VariableHeaderFactory) -> Message {
    assert!(
        !subs.is_empty(),
        "SUBSCRIBE packet must contain at least one topic"
    );

    let header: u16 = 0x82;

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
        fixed_header: header,
        var_header: var_header,
        payload: payload,
    }
}

fn create_unsubscribe(topics: &[&str], id: u16, factory: &VariableHeaderFactory) -> Message {
    let header: u16 = 0x82;

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
        fixed_header: header,
        var_header,
        payload,
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

    // Byte 1: Type (0x30 / 3 << 4) | DUP (bit 3) | QoS (bits 2-1) | RETAIN (bit 0)
    let byte_1: u8 = (3 << 4) | (dup_bit << 3) | (qos_bits << 1) | retain_bit;
    let remaining_length = var_header.len() + payload.len();
    let fixed_header: u16 = ((byte_1 as u16) << 8) | (remaining_length as u16);

    Message {
        fixed_header,
        var_header,
        payload,
    }
}

fn create_puback(identifier: u16) -> Message {
    Message {
        fixed_header: 0x4002,
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
    }
}
fn create_pubrec(identifier: u16) -> Message {
    Message {
        fixed_header: 0x5002,
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
    }
}
fn create_pubrel(identifier: u16) -> Message {
    Message {
        fixed_header: 0x6202,
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
    }
}
fn create_pubcomp(identifier: u16) -> Message {
    Message {
        fixed_header: 0x7002,
        var_header: identifier.to_be_bytes().to_vec(),
        payload: Vec::new(),
    }
}

fn create_disconnect() -> Message {
    Message {
        fixed_header: 0xE000,
        var_header: Vec::new(),
        payload: Vec::new(),
    }
}

fn create_pingreq() -> Message {
    Message {
        fixed_header: 0xC000,
        var_header: Vec::new(),
        payload: Vec::new(),
    }
}

pub fn create_message(msg_type: Type, factory: &VariableHeaderFactory) -> Message {
    match msg_type {
        Type::CONNECT(client_id, username, pw, will) => {
            return create_connect(client_id, username, pw, will, &factory);
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

pub fn parse_message(data: &[u8], factory: &VariableHeaderFactory) -> Message {
    // tcp receive
    // decode the payload of tcp
    // do shit with the payload starting
    // by parsing the msgtype
    //
    //
    let msg_type: Type = Type::PINGREQ;
    match msg_type {
        Type::CONNACK => {}
        Type::PUBACK(id) => {
            create_puback(id);
        }
        Type::PUBREC(id) => {
            create_pubrec(id);
        }
        Type::PUBREL(id) => {
            create_pubrel(id);
        }
        Type::PUBCOMP(id) => {
            create_pubcomp(id);
        }
        Type::SUBACK(id) => {}
        Type::UNSUBACK(id) => {}
        Type::PINGRESP => {}
        Type::PUBLISH(topic, id, dup, qos, retain, payload) => {}
        _ => {}
    }

    Message {
        fixed_header: 0x0,
        var_header: Vec::new(),
        payload: Vec::new(),
    }
}
