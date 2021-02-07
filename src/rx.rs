use anyhow::Result;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::Mutex,
};

#[derive(Debug)]
pub struct Rx {
    run: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
    t_rx_data: Arc<Mutex<Vec<f64>>>,
    t_diff_data: Arc<Mutex<Vec<f32>>>,
}

impl Drop for Rx {
    fn drop(&mut self) {
        self.run.store(false, Ordering::SeqCst);
        self.join_handle.take().unwrap().join().ok();
    }
}

impl Rx {
    pub fn new(bind_address: &str, bind_port: u16) -> Result<Rx> {
        let run = Arc::new(AtomicBool::new(true));
        let run_thread = run.clone();

        let bind_addr = SocketAddr::new(IpAddr::from_str(&bind_address).expect("error"), bind_port);

        let sock = UdpSocket::bind(bind_addr)?;
        sock.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut rx_buffer: Vec<u8> = Vec::with_capacity(9048);
        rx_buffer.resize(9048, 0);

        let mut last_rx_time = UNIX_EPOCH;

        let t_rx_data: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
        let t_rx_data_local = t_rx_data.clone();

        let t_diff_data: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let t_diff_data_local = t_diff_data.clone();

        let join_handle = thread::spawn(move || {
            while run_thread.load(Ordering::SeqCst) {
                match sock.recv_from(rx_buffer.as_mut_slice()) {
                    Ok((_received, _from)) => {
                        let rx_time = SystemTime::now();
                        let since_the_epoch = rx_time
                            .duration_since(UNIX_EPOCH)
                            .expect("error converting time");
                        let since_last = rx_time
                            .duration_since(last_rx_time)
                            .expect("error converting time");

                        if let Some(mut t_rx_data) = t_rx_data_local.lock().ok() {
                            t_rx_data.push(since_the_epoch.as_secs_f64());
                        }

                        if last_rx_time > UNIX_EPOCH {
                            if let Some(mut t_diff_data) = t_diff_data_local.lock().ok() {
                                t_diff_data.push(since_last.as_secs_f32());
                            }
                        }

                        last_rx_time = rx_time;
                    }
                    Err(ref e) if e.kind() != std::io::ErrorKind::TimedOut => {
                        println!("sock recv_from failed: {:?}, len: {}", e, rx_buffer.len())
                    }
                    Err(_) => (),
                }
            }
        });

        Ok(Rx {
            run: run,
            join_handle: Some(join_handle),
            t_rx_data,
            t_diff_data,
        })
    }

    pub fn get_t_diff_data(&self) -> Vec<f32> {
        if let Some(t_diff_data) = self.t_diff_data.lock().ok() {
            return t_diff_data.clone();
        }
        return Vec::new();
    }

    pub fn get_t_rx_data(&self) -> Vec<f64> {
        if let Some(t_rx_data) = self.t_rx_data.lock().ok() {
            return t_rx_data.clone();
        }
        return Vec::new();
    }
}
