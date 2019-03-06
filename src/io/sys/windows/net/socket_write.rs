use std::io;
use std::os::windows::io::{AsRawSocket, FromRawSocket, IntoRawSocket};
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::super::{co_io_result, EventData};
use coroutine_impl::{CoroutineImpl, EventSource};
use miow::net::TcpStreamExt;
use scheduler::get_scheduler;
use winapi::shared::ntdef::*;

pub struct SocketWrite<'a> {
    io_data: EventData,
    buf: &'a [u8],
    socket: ::std::net::TcpStream,
    timeout: Option<Duration>,
}

impl<'a> SocketWrite<'a> {
    pub fn new<T: AsRawSocket>(s: &T, buf: &'a [u8], timeout: Option<Duration>) -> Self {
        let socket = s.as_raw_socket();
        SocketWrite {
            io_data: EventData::new(socket as HANDLE),
            buf,
            socket: unsafe { FromRawSocket::from_raw_socket(socket) },
            timeout,
        }
    }

    #[inline]
    pub fn done(self) -> io::Result<usize> {
        // don't close the socket
        self.socket.into_raw_socket();
        co_io_result(&self.io_data)
    }
}

impl<'a> EventSource for SocketWrite<'a> {
    fn subscribe(&mut self, co: CoroutineImpl) {
        let s = get_scheduler();
        s.get_selector()
            .add_io_timer(&mut self.io_data, self.timeout);
        // prepare the co first
        self.io_data.co.swap(co, Ordering::Release);
        // call the overlapped write API
        co_try!(s, self.io_data.co.take(Ordering::AcqRel), unsafe {
            self.socket
                .write_overlapped(self.buf, self.io_data.get_overlapped())
        });
    }
}
