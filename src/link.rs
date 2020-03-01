use anyhow::Result;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LinkMode {
    Tx,
    Rx,
}

#[derive(Debug)]
pub struct Link {
    pub run: Arc<AtomicBool>,
    pub thread: Option<JoinHandle<()>>,
    pub link_mode: LinkMode,
    pub address: String,
    pub bind_port: u16,
    pub target_port: u16,
    pub packet_size: i32,
    pub send_interval_us: i32,
}

#[derive(Debug)]
pub struct LinkPacketData {
    t: f64,
    payload_size: i32,
}

impl Link {
    pub fn new(
        link_mode: LinkMode,
        address: &str,
        bind_port: u16,
        target_port: u16,
        packet_size: i32,
        tx: mpsc::Sender<LinkPacketData>,
        send_interval_us: i32,
    ) -> Result<Link> {
        let run = Arc::new(AtomicBool::new(true));
        let run_thread = run.clone();

        let bind_addr = SocketAddr::new(IpAddr::from_str(&address).expect("error"), bind_port);
        let target_addr = SocketAddr::new(IpAddr::from_str(&address).expect("error"), target_port);

        let sock = UdpSocket::bind(bind_addr)?;
        sock.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut payload: Vec<u8> = Vec::with_capacity(9048);
        match link_mode {
            LinkMode::Tx => {
                payload.resize_with(packet_size as usize, Default::default);
            }
            LinkMode::Rx => {
                payload.resize_with(9048, Default::default);
            }
        }

        let thread = thread::spawn(move || {
            let begin = Instant::now();

            let mut next_tx_time_us = send_interval_us;

            while run_thread.load(Ordering::SeqCst) {
                match link_mode {
                    LinkMode::Tx => {
                        if Instant::now().saturating_duration_since(begin)
                            > Duration::from_micros(next_tx_time_us as u64)
                        {
                            println!(
                                "Socket send took too much time? ({} > {})",
                                Instant::now().saturating_duration_since(begin).as_micros()
                                    - Duration::from_micros(next_tx_time_us as u64).as_micros(),
                                next_tx_time_us
                            );
                        }

                        while Instant::now().saturating_duration_since(begin)
                            < Duration::from_micros(next_tx_time_us as u64)
                            && run_thread.load(Ordering::SeqCst)
                        {}

                        let tx_time = SystemTime::now();
                        let since_the_epoch = tx_time
                            .duration_since(UNIX_EPOCH)
                            .expect("error converting time");

                        next_tx_time_us += send_interval_us;

                        sock.send_to(&payload, target_addr).ok();

                        tx.send(LinkPacketData {
                            t: since_the_epoch.as_secs_f64(),
                            payload_size: packet_size,
                        })
                        .expect("error sending data on channel");
                    }
                    LinkMode::Rx => match sock.recv_from(payload.as_mut_slice()) {
                        Ok((received, _from)) => {
                            let rx_time = SystemTime::now();
                            let since_the_epoch = rx_time
                                .duration_since(UNIX_EPOCH)
                                .expect("error converting time");
                            tx.send(LinkPacketData {
                                t: since_the_epoch.as_secs_f64(),
                                payload_size: received as i32,
                            })
                            .expect("error sending data on channel");
                        }
                        Err(ref e) if e.kind() != std::io::ErrorKind::TimedOut => {
                            println!("sock recv_from failed: {:?}", e)
                        }
                        Err(_) => (),
                    },
                }
            }
        });

        Ok(Link {
            run: run,
            thread: Some(thread),
            link_mode: link_mode,
            address: address.to_owned(),
            bind_port: bind_port,
            target_port: target_port,
            packet_size: packet_size,
            send_interval_us: send_interval_us,
        })
    }
}
