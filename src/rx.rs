use anyhow::Result;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Rx {
    run: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Rx {
    pub fn new(bind_address: &str, bind_port: u16) -> Result<Rx> {
        let run = Arc::new(AtomicBool::new(true));
        let run_thread = run.clone();

        let bind_addr = SocketAddr::new(IpAddr::from_str(&bind_address).expect("error"), bind_port);

        let sock = UdpSocket::bind(bind_addr)?;
        sock.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut rx_buffer: Vec<u8> = Vec::with_capacity(9048);

        let thread = thread::spawn(move || {
            while run_thread.load(Ordering::SeqCst) {
                match sock.recv_from(rx_buffer.as_mut_slice()) {
                    Ok((received, _from)) => {
                        let rx_time = SystemTime::now();
                        let since_the_epoch = rx_time
                            .duration_since(UNIX_EPOCH)
                            .expect("error converting time");
                        /*tx.send(LinkPacketData {
                            t: since_the_epoch.as_secs_f64(),
                            payload_size: received as i32,
                        })
                        .expect("error sending data on channel");*/
                    }
                    Err(ref e) if e.kind() != std::io::ErrorKind::TimedOut => {
                        println!("sock recv_from failed: {:?}", e)
                    }
                    Err(_) => (),
                }
            }
        });

        Ok(Rx {
            run: run,
            thread: Some(thread),
        })
    }
}
