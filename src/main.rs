use std::convert::TryInto;
use std::env;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;

use rand::Rng;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let mut rng = rand::thread_rng();

    let args: Vec<String> = env::args().collect();

    let send_interval_us = 1000;
    let packet_burst_count = 10;
    let magic: u64 = 0x875f9cdaf0bf51cc;

    if args.len() == 3 {
        let mode: &str = &args[1];

        match mode.as_ref() {
            "tx" => {
                let parts = &args[2].split(':').collect::<Vec<&str>>();
                let bind_address;
                let destination_ip;
                let destination_port;
                let parts_ok;
                if parts.len() == 2 {
                    bind_address = "0.0.0.0";
                    destination_ip = parts[0];
                    destination_port = parts[1];
                    parts_ok = true;
                } else if parts.len() == 3 {
                    bind_address = parts[0];
                    destination_ip = parts[1];
                    destination_port = parts[2];
                    parts_ok = true;
                } else {
                    bind_address = "";
                    destination_ip = "";
                    destination_port = "";
                    parts_ok = false;
                }

                if parts_ok {
                    println!(
                        "tx: {}:{}:{}",
                        &bind_address, &destination_ip, &destination_port
                    );

                    let mut socket = UdpSocket::bind(format!("{}:0", &bind_address))
                        .await
                        .expect("Couldn't bind to address");
                    socket
                        .connect(format!("{}:{}", &destination_ip, &destination_port))
                        .await
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

                        buf.resize(rng.gen_range(64, 8400), 0);

                        while Instant::now().saturating_duration_since(begin)
                            < Duration::from_millis(next_action_time_ms)
                        {}

                        next_action_time_ms += 1;

                        for _ in 0..packet_burst_count {
                            buf[0..8].copy_from_slice(&magic.to_le_bytes());
                            buf[8..16].copy_from_slice(&packet_number.to_le_bytes());

                            socket.send(&buf).await?;

                            packet_number += 1;
                        }
                    }
                }
            }
            "rx" => {
                let parts = &args[2].split(':').collect::<Vec<&str>>();
                if parts.len() == 2 {
                    let device_id = parts[0];
                    let port = parts[1];

                    match pcap::Device::list() {
                        Ok(devices) => {
                            for device in devices {
                                if device.name.contains(device_id) {
                                    println!("Rx using device: {} {:?}", device.name, device.desc);

                                    let mut cap = pcap::Capture::from_device(device)
                                        .unwrap()
                                        .timeout(1000000000)
                                        .buffer_size(512 * 1024 * 1024)
                                        .open()
                                        .unwrap();
                                    cap.filter(&format!("dst port {}", port)).unwrap();

                                    let mut last_rx_time = Instant::now();

                                    let mut last_packet_nr: u64 = 0;

                                    let mut average_time_between_rx_s = 0.0;
                                    let mut max_time_between_rx_s = 0.0;
                                    let mut acc_count = 0;
                                    let mut reorders = 0;
                                    let mut samples_above_2 = 0;
                                    let mut samples_above_4 = 0;
                                    let mut samples_above_8 = 0;
                                    let mut samples_above_16 = 0;

                                    while let Ok(packet) = cap.next() {
                                        if packet.data.len() > 58 {
                                            let data: &[u8] = &packet.data[42..];

                                            let rx_magic =
                                                u64::from_le_bytes(data[0..8].try_into().unwrap());
                                            let packet_nr =
                                                u64::from_le_bytes(data[8..16].try_into().unwrap());

                                            if rx_magic == magic {
                                                if last_packet_nr != 0
                                                    && last_packet_nr + 1 != packet_nr
                                                {
                                                    reorders += 1;
                                                }
                                                last_packet_nr = packet_nr;

                                                let now = Instant::now();

                                                let time_since_last_rx_s = now
                                                    .saturating_duration_since(last_rx_time)
                                                    .as_secs_f64();

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

                                                acc_count += 1;

                                                let acc_max = 100_000;

                                                if acc_count >= acc_max {
                                                    let average_ms = 1000.0
                                                        * (average_time_between_rx_s
                                                            / (acc_max as f64));
                                                    let max_ms = max_time_between_rx_s * 1000.0;
                                                    println!(
                                                    "Stats for last {} samples, average time between rx: {:.01} ms, max: {:.01} ms, above 2 ms: {}, above 4 ms: {}, above 8 ms: {}, above 16 ms: {}, reorders: {}",
                                                    acc_max,
                                                    average_ms,
                                                    max_ms,
                                                    samples_above_2,
                                                    samples_above_4,
                                                    samples_above_8,
                                                    samples_above_16,
                                                    reorders);

                                                    average_time_between_rx_s = 0.0;
                                                    max_time_between_rx_s = 0.0;
                                                    acc_count = 0;
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

                                    println!("Unexpected end of packet stream");
                                    return Ok(());
                                }
                            }
                        }
                        Err(e) => println!("Error listing network devices: {:?}", e),
                    }

                    println!("No device found matching device id: {}", device_id);
                }
            }
            &_ => {}
        }
    }

    println!("This program will either send {} udp packets every {} μs, or listen for packets and print stats.", packet_burst_count, send_interval_us);
    println!(
        "To use, supply arguments: tx ([bind_ip]:)[target_ip]:[port] or: rx [interface]:[port]"
    );
    println!(
        "Available network interfaces (it is enough to specify part of the uuid of an interface)"
    );

    match pcap::Device::list() {
        Ok(devices) => {
            for device in devices {
                println!("{}, {:?}", device.name, device.desc);
            }
        }
        Err(e) => println!("error listing devices: {:?}", e),
    }

    Ok(())
}
