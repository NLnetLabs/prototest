//! Streams sending and receiving sequences of data.

use std::io;
use std::io::{Read, Write};
use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
#[cfg(feature = "tokio")]
use {
    std::pin::Pin,
    std::task::{Context, Poll},
    tokio::io::{AsyncRead, AsyncWrite, ReadBuf},
};

//------------ AssertStream --------------------------------------------------

/// A stream that sends data and expects certain values in return.
///
/// This stream is intended to be used instead of, e.g., `TcpStream` in a
/// generic protocol implementation. Thus, its rules are to be viewed from
/// the perspective of that protcol implemenatation, _not_ from the
/// perspective of the other end of the conversation.
#[derive(Clone, Debug)]
pub struct AssertStream {
    /// The rules that drive this stream.
    rules: AssertRules,

    /// The index of the current rule.
    rule_index: usize,

    /// The index of the data of a send all or recv all rule.
    all_index: usize,
}

impl AssertStream {
    pub fn new(rules: AssertRules) -> Self {
        AssertStream {
            rules,
            rule_index: 0,
            all_index: 0
        }
    }

    pub fn from_ron_str(s: &str) -> Result<Self, ron::error::Error> {
        ron::de::from_str(s).map(Self::new)
    }

    fn next_fragment(&mut self) {
        self.rule_index += 1;
        self.all_index = 0;
    }
}

impl Read for AssertStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, io::Error> {
        match self.rules.fragments.get(self.rule_index) {
            Some(FragmentRule::Send(_)) | Some(FragmentRule::SendAll(_)) => {
                Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "expected send"
                ))
            }
            Some(FragmentRule::Recv(ref data)) => {
                let len = data.len();
                if buf.len() < len {
                    panic!("short buffer provided")
                }
                buf[..len].copy_from_slice(data);
                self.next_fragment();
                Ok(len)
            }
            Some(FragmentRule::RecvAll(ref data)) => {
                let remaining_data = &data[self.all_index..];
                let len = remaining_data.len();
                let buf_remaining = buf.len();
                if buf_remaining >= remaining_data.len() {
                    buf[..remaining_data.len()].copy_from_slice(
                        remaining_data
                    );
                    self.next_fragment();
                    Ok(len)
                }
                else {
                    buf.copy_from_slice(&remaining_data[..buf_remaining]);
                    self.all_index += buf_remaining;
                    Ok(buf_remaining)
                }
            }
            Some(FragmentRule::SendClose) => {
                panic!("Expected send close.")
            }
            Some(FragmentRule::RecvClose) => {
                Ok(0)
            }
            None => {
                panic!("no more fragement rules")
            }
        }
    }
}

#[cfg(feature = "tokio")]
impl AsyncRead for AssertStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), io::Error>> {
        match self.rules.fragments.get(self.rule_index) {
            Some(FragmentRule::Send(_)) | Some(FragmentRule::SendAll(_)) => {
                Poll::Pending
            }
            Some(FragmentRule::Recv(ref data)) => {
                buf.put_slice(data);
                self.next_fragment();
                Poll::Ready(Ok(()))
            }
            Some(FragmentRule::RecvAll(ref data)) => {
                let remaining_data = &data[self.all_index..];
                let buf_remaining = buf.remaining();
                if buf_remaining >= remaining_data.len() {
                    buf.put_slice(remaining_data);
                    self.next_fragment();
                }
                else {
                    buf.put_slice(&remaining_data[..buf_remaining]);
                    self.all_index += buf_remaining;
                }
                Poll::Ready(Ok(()))
            }
            Some(FragmentRule::SendClose) => {
                panic!("Expected send close.")
            }
            Some(FragmentRule::RecvClose) => {
                Poll::Ready(Ok(()))
            }
            None => {
                panic!("no more fragement rules")
            }
        }
    }
}

impl Write for AssertStream {
    fn write(&mut self, mut buf: &[u8]) -> Result<usize, io::Error> {
        match self.rules.fragments.get(self.rule_index) {
            Some(FragmentRule::Send(ref data)) => {
                if buf.len() > data.len() {
                    buf = &buf[..data.len()];
                }
                assert_eq!(buf, data);
                self.next_fragment();
                Ok(buf.len())
            }
            Some(FragmentRule::SendAll(ref full_data)) => {
                let mut data = &full_data[self.all_index..];
                match buf.len().cmp(&data.len()) {
                    Ordering::Greater => {
                        buf = &buf[..data.len()];
                    }
                    Ordering::Less => {
                        data = &data[..buf.len()];
                        self.all_index += buf.len();
                    }
                    Ordering::Equal => { }
                }
                assert_eq!(buf, data);
                self.all_index += buf.len();
                if self.all_index == full_data.len() {
                    self.next_fragment();
                }
                Ok(buf.len())
            }
            Some(FragmentRule::Recv(_)) | Some(FragmentRule::RecvAll(_)) => {
                panic!("expected recv")
            }
            Some(FragmentRule::SendClose) => panic!("expected send close"),
            Some(FragmentRule::RecvClose) => panic!("expected recv close"),
            None => {
                panic!("no more fragement rules")
            }
        }
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}

#[cfg(feature = "tokio")]
impl AsyncWrite for AssertStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8]
    ) -> Poll<Result<usize, io::Error>> {
        Poll::Ready(self.write(buf))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>
    ) -> Poll<Result<(), io::Error>> {
        match self.rules.fragments.get(self.rule_index) {
            Some(FragmentRule::SendClose) => Poll::Ready(Ok(())),
            _ => panic!("expected send close")
        }
    }
}



//------------ AssertRules ---------------------------------------------------

/// The rules an followed by an assert stream.
///
/// The type is generic over the type of some associated data. This allows you
/// to define both the stream’s conversation and whatever data should result
/// from this conversation in one common place.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AssertRules<Asoc = ()> {
    pub fragments: Vec<FragmentRule>,
    pub associated: Asoc,
}


//------------ FragmentRule --------------------------------------------------

/// A rule for sending or receiving a fragment of data.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FragmentRule {
    /// A packet should be sent.
    ///
    /// An `AssertStream` expects the protocol implementation to send the
    /// provided data via `AsyncWrite` or `Write`. If the data sent differs
    /// from the data provided, the `AssertStream` will panic.
    ///
    /// If the protocol implementation tries to read, this will fail. An
    /// async read will simply return `Poll::Pending`. A sync read will
    /// return a would-block error.
    ///
    /// If the buffer sent is longer than the given data but starts with the
    /// given data, the write will return the length of the given data as
    /// written.
    Send(Vec<u8>),

    /// Data should be sent through a sequence of packets.
    ///
    /// This is similar to `Send(_)` except that the data may be sent through
    /// a sequence of packets.
    SendAll(Vec<u8>),

    /// A packet should be received.
    ///
    /// If the protocol implementation tries to read, it will receive the
    /// provided data. If the protocol implementation tries to write, the
    /// `AssertStream` will panic.
    ///
    /// If the buffer provided is too short, panics.
    Recv(Vec<u8>),

    /// Data should be read through a sequence of packets.
    ///
    /// This is similar to `Recv(_)` except that the data may be read through
    /// a sequence of packets.
    RecvAll(Vec<u8>),

    /// The protocol implementation should close the stream.
    ///
    /// Any reading or writing will cause a panic.
    SendClose,

    /// The connection should be closed by the peer.
    ///
    /// If the protocol implementation tries to read, it will receive a
    /// zero-sized packet. If it tries to write, the `AssertStream` will
    /// panic.
    RecvClose,
}


//============ Tests ========================================================

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn simple() {
        let mut stream = AssertStream::from_ron_str(r#"
            AssertRules(
                fragments: [
                    Recv([0x20, 0x20, 0x20]),
                    Send([0x20, 0x20, 0x20]),
                    RecvClose,
                ]
            )
        "#).unwrap();
        let mut buf = vec![0; 5];
        assert_eq!(stream.read(&mut buf).unwrap(), 3);
        assert_eq!(&buf[..3], b"\x20\x20\x20");
        assert_eq!(stream.write(b"\x20\x20\x20").unwrap(), 3);
        assert_eq!(stream.read(&mut buf).unwrap(), 0);
    }
}

