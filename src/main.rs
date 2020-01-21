use std::env;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut print_usage_instructions = args.len() != 3;

    let send_interval_us = 1000;
    let packet_size_bytes = 500;

    if !print_usage_instructions {
        let mode: &str = &args[1];

        match mode.as_ref() {
            "tx" => {
                let destination = &args[2];

                println!("sending to {}", destination);

                let socket = UdpSocket::bind("0.0.0.0:0").expect("Couldn't bind to address");
                socket.connect(destination).expect("connection failed");
                let begin = Instant::now();
                let mut next_action_time_ms = 1;

                let mut buf: Vec<u8> = Vec::new();
                buf.resize(packet_size_bytes, 0);

                loop {
                    while Instant::now().saturating_duration_since(begin)
                        < Duration::from_millis(next_action_time_ms)
                    {}

                    next_action_time_ms += 1;

                    socket.send(&buf)?;
                }
            }
            "rx" => {
                let listen_port = args[2]
                    .parse::<u16>()
                    .expect("Failed to parse destination port");

                let socket = UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], listen_port)))?;
                socket
                    .set_read_timeout(None)
                    .expect("set_read_timeout call failed");

                let mut buf = [0; 9000];

                let mut last_rx_time = Instant::now();
                let begin = Instant::now();
                loop {
                    let (number_of_bytes, src_addr) = socket.recv_from(&mut buf)?;

                    let now = Instant::now();
                    println!(
                        "{:?};{:?};{};{}",
                        now.saturating_duration_since(begin),
                        now.saturating_duration_since(last_rx_time),
                        number_of_bytes,
                        src_addr
                    );
                    last_rx_time = now;
                }
            }
            &_ => {
                print_usage_instructions = true;
            }
        }
    }

    if print_usage_instructions {
        println!(
            "This program will either send a {} b udp packet every {} μs or listen for packets and print the time diff.
To use, supply arguments: tx [target_ip:port] or: rx [listen_port]", packet_size_bytes, send_interval_us);
    }

    Ok(())
}
