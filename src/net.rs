use std::net::TcpStream;

pub fn connect(ip: &str) -> std::io::Result<TcpStream> {
    TcpStream::connect(ip);
}
