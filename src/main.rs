use std::convert::TryInto;
use std::env;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

use rand::Rng;

fn main() -> std::io::Result<()> {
    let mut rng = rand::thread_rng();

    let args: Vec<String> = env::args().collect();

    let mut print_usage_instructions = args.len() != 3;

    let send_interval_us = 1000;
    let packet_size_bytes = 500;

    if !print_usage_instructions {
        let mode: &str = &args[1];

        match mode.as_ref() {
            "tx" => {
                let parts = &args[2].split(':').collect::<Vec<&str>>();
                let bind_address;
                let destination_ip;
                let destination_port;
                if parts.len() == 2 {
                    bind_address = "0.0.0.0";
                    destination_ip = parts[0];
                    destination_port = parts[1];
                } else if parts.len() == 3 {
                    bind_address = parts[0];
                    destination_ip = parts[1];
                    destination_port = parts[2];
                } else {
                    bind_address = "";
                    destination_ip = "";
                    destination_port = "";
                    print_usage_instructions = true;
                }

                if !print_usage_instructions {
                    println!(
                        "tx: {}:{}:{}",
                        &bind_address, &destination_ip, &destination_port
                    );

                    let socket = UdpSocket::bind(format!("{}:0", &bind_address))
                        .expect("Couldn't bind to address");
                    socket
                        .connect(format!("{}:{}", &destination_ip, &destination_port))
                        .expect("connection failed");
                    let begin = Instant::now();
                    let mut next_action_time_ms = 1;

                    let mut buf: Vec<u8> = Vec::new();

                    let mut packet_number: u64 = 0;

                    loop {
                        if Instant::now().saturating_duration_since(begin)
                            > Duration::from_millis(next_action_time_ms)
                        {
                            println!(
                                "Socket send took {:.02} ms longer than expected",
                                (Instant::now().saturating_duration_since(begin)
                                    - Duration::from_millis(next_action_time_ms))
                                .as_secs_f32()
                                    * 1000.0
                            );
                        }

                        buf.resize(rng.gen_range(10, 8400), 0);

                        while Instant::now().saturating_duration_since(begin)
                            < Duration::from_millis(next_action_time_ms)
                        {}

                        next_action_time_ms += 1;

                        buf[0..8].copy_from_slice(&packet_number.to_le_bytes());

                        socket.send(&buf)?;

                        packet_number += 1;
                    }
                }
            }
            "rx" => {
                let parts = &args[2].split(':').collect::<Vec<&str>>();
                let bind_address;
                let listen_port;
                if parts.len() == 1 {
                    bind_address = "0.0.0.0";
                    listen_port = parts[0];
                } else if parts.len() == 2 {
                    bind_address = parts[0];
                    listen_port = parts[1];
                } else {
                    bind_address = "";
                    listen_port = "";
                    print_usage_instructions = true;
                }

                if !print_usage_instructions {
                    let socket = UdpSocket::bind(format!("{}:{}", &bind_address, &listen_port))
                        .expect("Couldn't bind to address");
                    socket
                        .set_read_timeout(None)
                        .expect("set_read_timeout call failed");

                    let mut buf = [0; 9000];

                    let mut last_rx_time = Instant::now();

                    let mut last_packet_nr: u64 = 0;

                    let mut average_time_between_rx_s = 0.0;
                    let mut max_time_between_rx_s = 0.0;
                    let mut acc_times = 0;
                    let mut reorders = 0;
                    let mut samples_above_2 = 0;
                    let mut samples_above_4 = 0;
                    let mut samples_above_8 = 0;
                    let mut samples_above_16 = 0;

                    loop {
                        let (number_of_bytes, _src_addr) = socket.recv_from(&mut buf)?;

                        if number_of_bytes > 8 {
                            let packet_nr = u64::from_le_bytes(buf[0..8].try_into().unwrap());

                            if last_packet_nr != 0 && last_packet_nr + 1 != packet_nr {
                                reorders += 1;
                            }
                            last_packet_nr = packet_nr;

                            let now = Instant::now();

                            let time_since_last_rx_s =
                                now.saturating_duration_since(last_rx_time).as_secs_f64();

                            average_time_between_rx_s += time_since_last_rx_s;
                            if max_time_between_rx_s < time_since_last_rx_s {
                                max_time_between_rx_s = time_since_last_rx_s;
                            }

                            if time_since_last_rx_s >= 0.002 {
                                samples_above_2 += 1;
                            }
                            if time_since_last_rx_s >= 0.004 {
                                samples_above_4 += 1;
                            }
                            if time_since_last_rx_s >= 0.008 {
                                samples_above_8 += 1;
                            }
                            if time_since_last_rx_s >= 0.016 {
                                samples_above_16 += 1;
                            }

                            acc_times += 1;

                            if acc_times >= 10000 {
                                let average_ms = average_time_between_rx_s / 10.0;
                                let max_ms = max_time_between_rx_s * 1000.0;
                                println!(
                                    "Stats for last 10'000 samples, average time between rx: {:.01} ms, max: {:.01} ms, above 2 ms: {}, above 4 ms: {}, above 8 ms: {}, above 16 ms: {}, reorders: {}",
                                    average_ms,
                                    max_ms,
                                    samples_above_2,
                                    samples_above_4,
                                    samples_above_8,
                                    samples_above_16,
                                    reorders);

                                average_time_between_rx_s = 0.0;
                                acc_times = 0;
                                max_time_between_rx_s = 0.0;
                                acc_times = 0;
                                reorders = 0;
                                samples_above_2 = 0;
                                samples_above_4 = 0;
                                samples_above_8 = 0;
                                samples_above_16 = 0;
                            }
                            last_rx_time = now;
                        }
                    }
                }
            }
            &_ => {
                print_usage_instructions = true;
            }
        }
    }

    if print_usage_instructions {
        println!("This program will either send a {} b udp packet every {} μs or listen for packets and print the time diff.", packet_size_bytes, send_interval_us);
        println!("To use, supply arguments: tx ([bind_ip]:)[target_ip]:[port] or: rx ([bind_ip]:)[listen_port]");
    }

    Ok(())
}
