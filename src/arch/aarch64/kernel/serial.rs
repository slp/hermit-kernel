use core::arch::asm;

use crate::syscalls::interfaces::serial_buf_hypercall;

enum SerialInner {
	Pl011(u32),
	Uart(u32),
	Uhyve,
}

pub struct SerialPort {
	inner: SerialInner,
}

impl SerialPort {
	pub fn new(port_address: u32) -> Self {
		if crate::env::is_uhyve() {
			Self {
				inner: SerialInner::Uhyve,
			}
		} else {
			Self {
				inner: SerialInner::Pl011(port_address),
			}
		}
	}

	fn pl011_write_char(port_address: u32, byte: u8) {
		let dr = (port_address + 0x0) as *mut u8;
		let fr = (port_address + 0x18) as *mut u32;

		unsafe {
			loop {
				if (fr.read_volatile() & 0x20) == 0 {
					break;
				}
			}

			dr.write_volatile(byte.into());
		}
	}

	pub fn write_buf(&mut self, buf: &[u8]) {
		match &mut self.inner {
			SerialInner::Uhyve => {
				serial_buf_hypercall(buf);
			}
			SerialInner::Pl011(port_address) => {
				for &byte in buf {
					// LF newline characters need to be extended to CRLF over a real serial port.
					if byte == b'\n' {
						SerialPort::pl011_write_char(*port_address, b'\r');
					}
					SerialPort::pl011_write_char(*port_address, byte);
				}
			}
			SerialInner::Uart(port_address) => {
				let port = core::ptr::with_exposed_provenance_mut::<u8>(*port_address as usize);
				for &byte in buf {
					// LF newline characters need to be extended to CRLF over a real serial port.
					if byte == b'\n' {
						unsafe {
							asm!(
								"strb w8, [{port}]",
								port = in(reg) port,
								in("x8") b'\r',
								options(nostack),
							);
						}
					}

					unsafe {
						asm!(
							"strb w8, [{port}]",
							port = in(reg) port,
							in("x8") byte,
							options(nostack),
						);
					}
				}
			}
		}
	}

	pub fn init(&self, _baudrate: u32) {
		// We don't do anything here (yet).
	}
}
