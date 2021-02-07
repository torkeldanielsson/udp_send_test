use anyhow::Result;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use std::{str::FromStr, sync::atomic::AtomicUsize};

#[derive(Debug)]
pub struct Tx {
    run: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
    send_count: Arc<AtomicUsize>,
}

impl Drop for Tx {
    fn drop(&mut self) {
        self.run.store(false, Ordering::SeqCst);
        self.join_handle.take().unwrap().join().ok();
    }
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

        let send_count = Arc::new(AtomicUsize::new(0));
        let send_count_thread = send_count.clone();

        let join_handle = thread::spawn(move || {
            let begin = Instant::now();

            let mut next_tx_time_us = send_interval_us;

            while run_thread.load(Ordering::SeqCst) {
                while Instant::now().saturating_duration_since(begin)
                    < Duration::from_micros(next_tx_time_us as u64)
                    && run_thread.load(Ordering::SeqCst)
                {}

                next_tx_time_us += send_interval_us;

                sock.send_to(&payload, target_addr).ok();

                send_count_thread.fetch_add(1, Ordering::SeqCst);
            }
        });

        Ok(Tx {
            run: run,
            join_handle: Some(join_handle),
            send_count: send_count,
        })
    }

    pub fn get_send_count(&self) -> u64 {
        self.send_count.load(Ordering::SeqCst) as u64
    }
}
