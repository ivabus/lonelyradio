use std::{
	borrow::BorrowMut,
	io,
	net::{SocketAddr, TcpStream},
	sync::Arc,
};

pub(crate) enum Writer {
	Unencrypted(TcpStream),
	XorEncrypted(TcpStream, Arc<Vec<u8>>, u64),
}

impl io::Write for Writer {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		match self {
			Self::Unencrypted(s) => s.write(buf),
			Self::XorEncrypted(s, key, n) => {
				for mut k in buf.iter().copied() {
					k ^= key[*n as usize];
					*n += 1;
					*n %= key.len() as u64;
					s.write_all(&[k])?;
				}
				Ok(buf.len())
			}
		}
	}
	fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
		match self {
			Self::Unencrypted(s) => s.write_all(buf),
			Self::XorEncrypted(s, key, n) => s.write_all(
				&buf.iter()
					.borrow_mut()
					.copied()
					.map(|mut k| {
						k ^= key[*n as usize];
						*n += 1;
						*n %= key.len() as u64;
						k
					})
					// I don't like it
					.collect::<Vec<u8>>(),
			),
		}
	}
	fn flush(&mut self) -> io::Result<()> {
		match self {
			Self::XorEncrypted(s, _, _) | Self::Unencrypted(s) => s.flush(),
		}
	}
}

impl Writer {
	pub fn peer_addr(&self) -> io::Result<SocketAddr> {
		match self {
			Self::XorEncrypted(s, _, _) | Self::Unencrypted(s) => s.peer_addr(),
		}
	}
}
