use std::{io, net::TcpStream};

pub(crate) enum Reader {
	Unencrypted(TcpStream),
	XorEncrypted(TcpStream, Vec<u8>, u64),
}

impl io::Read for Reader {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match self {
			Self::Unencrypted(s) => s.read(buf),
			Self::XorEncrypted(s, key, n) => {
				let out = s.read(buf);
				if let Ok(i) = &out {
					for k in buf.iter_mut().take(*i) {
						*k ^= key[*n as usize];
						*n += 1;
						*n %= key.len() as u64;
					}
				}
				out
			}
		}
	}
}
