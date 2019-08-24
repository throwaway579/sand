#![allow(unused_imports)]

mod sendfile {
    use libc::sendfile;
    use libc::{c_int, off_t, size_t};
    use std::io::{Error, ErrorKind};
    use std::ptr;

    pub fn try_sendfile(
        file: c_int,
        stream: c_int,
        offset: off_t,
        mut length: off_t,
    ) -> Result<(), (Error, off_t)> {
        if unsafe {
            sendfile(
                file,
                stream,
                offset,
                &mut length as *mut off_t,
                ptr::null(),
                0,
            )
        } == -1
        {
            Err((Error::last_os_error(), length))
        } else {
            Ok(())
        }
    }
}

use sendfile::*;

use crate::fallback;

use libc::off_t;
use std::fs::File;
use std::io::{self, Error, ErrorKind};
use std::net::TcpStream;
use std::os::unix::io::AsRawFd;

#[cfg(not(feature = "large-files"))]
pub fn send_file(file: &mut File, stream: &mut TcpStream) -> io::Result<()> {
    let mut offset: off_t = 0;

    loop {
        // loop until the file has been sent and handle WouldBlock and Interrupted errors

        match try_sendfile(file.as_raw_fd(), stream.as_raw_fd(), offset, 0) {
            Err((ref e, sent)) if check_error(e.kind()) => {
                if e.kind() == ErrorKind::Interrupted && sent == 0 {
                    // special case
                    return Err(e.to_owned());
                }

                offset += sent;
            }
            other => return other.map_err(|(e, _)| e),
        };
    }
}

#[cfg(feature = "large-files")]
pub fn send_file(file: &mut File, stream: &mut TcpStream) -> io::Result<()> {
    let mut offset: off_t = 0;

    loop {
        match try_sendfile(file.as_raw_fd(), stream.as_raw_fd(), offset, 0) {
            Err((ref e, sent)) if check_error(e.kind()) => {
                if e.kind() == ErrorKind::Interrupted && sent == 0 {
                    return Err(e.to_owned());
                }

                let (new_offset, overflow) = offset.overflowing_add(sent);

                if overflow {
                    let offset = offset as u64 + sent as u64;

                    fallback::copy_to_end(file, stream, offset)?;
                } else {
                    // continue with the updated offset
                    offset = new_offset;
                }
            }
            other => return other.map_err(|(e, _)| e),
        };
    }
}

#[inline]
fn check_error(e: ErrorKind) -> bool {
    e == ErrorKind::WouldBlock || e == ErrorKind::Interrupted
}

pub fn send_exact(file: &File, stream: &TcpStream, length: u64, offset: u64) -> io::Result<u64> {
    #[cfg(feature = "large-files")]
    {
        if offset > off_t::max_value() as u64 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "offset exceeds maximum size",
            ));
        };
    }

    let length = if length > off_t::max_value() as u64 {
        off_t::max_value()
    } else {
        length as off_t
    };

    match try_sendfile(
        file.as_raw_fd(),
        stream.as_raw_fd(),
        offset as off_t,
        length,
    ) {
        Ok(()) => Ok(length as u64),
        Err((e, _)) => Err(e),
    }
}
