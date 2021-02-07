use anyhow::Result;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Tx {
    run: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Tx {
    pub fn new(
        bind_address: &str,
        target_address: &str,
        target_port: u16,
        packet_size: i32,
        send_interval_us: i32,
    ) -> Result<Tx> {
        let run = Arc::new(AtomicBool::new(true));
        let run_thread = run.clone();

        let bind_addr = SocketAddr::new(IpAddr::from_str(&bind_address).expect("error"), 0);
        let target_addr = SocketAddr::new(
            IpAddr::from_str(&target_address).expect("error"),
            target_port,
        );

        let sock = UdpSocket::bind(bind_addr)?;
        sock.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut payload: Vec<u8> = Vec::with_capacity(9048);
        payload.resize_with(packet_size as usize, Default::default);

        let thread = thread::spawn(move || {
            let begin = Instant::now();

            let mut next_tx_time_us = send_interval_us;

            while run_thread.load(Ordering::SeqCst) {
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
            }
        });

        Ok(Tx {
            run: run,
            thread: Some(thread),
        })
    }
}
