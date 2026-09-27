use std::io::Error;
use std::net::TcpStream;

pub fn connect(ip: &str) -> Result<TcpStream, Error> {
    TcpStream::connect(ip)
}
