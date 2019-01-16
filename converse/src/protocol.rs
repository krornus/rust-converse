use std::u32;
use std::mem;
use std::io::{self, prelude::*};

use crate::error::Error;

pub struct IPCRequest {
    pub key: u32,
    pub argc: u32,
    pub argv: Vec<IPCBuffer>,
}

pub struct IPCBuffer {
    pub len: u32,
    pub data: Vec<u8>,
}

impl IPCRequest {
    pub fn new(key: u32, argv: Vec<Vec<u8>>) -> Self {

        let argv: Vec<_> = argv.into_iter().map(IPCBuffer::new).collect();

        IPCRequest {
            key: key,
            argc: argv.len() as u32,
            argv: argv,
        }
    }

    pub fn read<T: Read>(stream: &mut T) -> Result<Self, Error> {

        let key = stream.read_u32()?;
        let argc = stream.read_u32()?;
        let mut argv = Vec::with_capacity(argc as usize);

        for _ in 0..argc {
            let buffer = IPCBuffer::read(stream)?;
            argv.push(buffer);
        }

        Ok(IPCRequest {
            key,
            argc,
            argv,
        })
    }

    pub fn write<T: Write>(self, stream: &mut T) -> Result<(), Error> {

        stream.write_u32(self.key)?;
        stream.write_u32(self.argc)?;

        assert!(self.argc == self.argv.len() as u32);

        for buf in self.argv {
            buf.write(stream)?;
        }

        Ok(())
    }
}

impl IPCBuffer {
    pub fn new(buf: Vec<u8>) -> Self {

        if buf.len() > u32::MAX as usize {
            panic!("IPCBuffer: given buffer greater than u32::MAX length");
        }

        IPCBuffer {
            len: buf.len() as u32,
            data: buf,
        }
    }

    pub fn read<T: Read>(stream: &mut T) -> Result<Self, Error> {
        let len = stream.read_u32()?;
        let mut buffer = vec![0; len as usize];

        stream.read_exact(&mut buffer[..])?;

        Ok(IPCBuffer {
            len: len,
            data: buffer,
        })
    }

    pub fn write<T: Write>(self, stream: &mut T) -> Result<(), Error> {

        if self.len as usize != self.data.len() {
            return Err(Error::Server(
                format!(
                    "IPCBuffer length mismatch: IPCBuffer::write tried to make a call with given length: {} and actual length: {}",
                    self.len, self.data.len()
                )))
        }

        stream.write_u32(self.len)?;
        stream.write_all(&self.data[..])?;

        Ok(())
    }
}

impl<T: Read> ReadU32 for T { }
impl<T: Write> WriteU32 for T { }
trait ReadU32 where Self: Read {
    fn read_u32(&mut self) -> Result<u32, io::Error> {

        let mut buf = [0_u8; 4];
        self.read_exact(&mut buf[..])?;

        let value = unsafe {
            mem::transmute::<[u8;4], u32>(buf)
        };

        Ok(value)
    }
}

trait WriteU32 where Self: Write {
    fn write_u32(&mut self, x: u32) -> Result<(), io::Error> {

        let buf = unsafe {
            mem::transmute::<u32, [u8;4]>(x)
        };

        self.write_all(&buf)?;

        Ok(())
    }
}
