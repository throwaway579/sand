#![allow(dead_code, unused_imports)]

use std::fs::File;
use std::io::{self, BufReader, ErrorKind, Read, Seek, SeekFrom, Write};
use std::net::TcpStream;

pub fn send_file(file: &mut File, stream: &mut TcpStream) -> io::Result<()> {
    let length = file.metadata()?.len();

    if length == 0 {
        return Ok(());
    };

    send_file_imp(file, stream, length)
}

pub fn send_exact(
    file: &mut File,
    stream: &mut TcpStream,
    length: u64,
    offset: u64,
) -> io::Result<u64> {
    file.seek(SeekFrom::Start(offset))?;
    io::copy(&mut file.take(length), stream)
}

#[cfg(not(any(feature = "fallback-bufreader", feature = "fallback-buf")))]
pub fn send_file_imp(file: &mut File, stream: &mut TcpStream, length: u64) -> io::Result<()> {
    let mut sent = io::copy(file, stream)?;

    while sent < length {
        sent += io::copy(file, stream)?;
    }

    Ok(())
}

#[cfg(feature = "fallback-bufreader")]
pub fn send_file_imp(file: &mut File, stream: &mut TcpStream, length: u64) -> io::Result<()> {
    let mut reader = BufReader::new(file);

    let mut sent = io::copy(&mut reader, stream)?;

    while sent < length {
        sent += io::copy(&mut reader, stream)?;
    }

    Ok(())
}

#[cfg(all(feature = "fallback-buf", not(feature = "large-files")))]
pub fn send_file_imp(file: &mut File, stream: &mut TcpStream, length: u64) -> io::Result<()> {
    let mut buf = Vec::with_capacity(length as usize);

    file.read_to_end(&mut buf)?;
    stream.write_all(&buf)?;

    Ok(())
}

#[cfg(all(feature = "fallback-buf", feature = "large-files"))]
pub fn send_file_imp(file: &mut File, stream: &mut TcpStream, length: u64) -> io::Result<()> {
    let mut remaining = length - usize::max_value() as u64;

    let mut buf = Vec::with_capacity(length as usize);

    file.read_to_end(&mut buf)?;
    stream.write_all(&buf)?;

    while remaining > 0 {
        remaining -= io::copy(file, stream)?;
    }

    Ok(())
}

pub fn copy_to_end(file: &mut File, stream: &mut TcpStream, offset: u64) -> io::Result<()> {
    file.seek(SeekFrom::Start(offset))?;

    loop {
        match io::copy(file, stream) {
            Ok(0) => return Ok(()),
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    return Err(e);
                }
            }
            _ => continue,
        }
    }
}
