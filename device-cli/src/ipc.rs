use anyhow::{Ok, Result};
use device_common::{Message, Packet, Request, SOCKET_PATH};
use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

pub struct Ipc {
    stream: UnixStream,
}

impl Ipc {
    pub fn new() -> Result<Self> {
        Ok(Self {
            stream: UnixStream::connect(SOCKET_PATH)?,
        })
    }

    pub fn request(&mut self, request: Request) -> Result<Option<Message>> {
        match request {
            Request::Get(get) => {
                self.stream.write_all(&get.to_packet())?;
                let mut response = Packet::default();
                self.stream.read_exact(&mut response)?;
                Ok(Some(Message::try_from(response)?))
            }
            Request::Set(set) => {
                self.stream.write_all(&set.to_packet())?;
                Ok(None)
            }
            Request::Subscribe => Ok(None),
        }
    }
}
