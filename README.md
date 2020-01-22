# udp_send_test

Usage: 
cargo run -- tx [target_ip:port] 
or: 
cargo run -- rx [listen_port]

This program can be used in two modes: rx or tx.

In tx mode it will send a 500 byte udp packet every millisecond to the given target ip and port.

In rx mode listen for packets and print, comma separated:
time since program start (ns)
time diff since last packet (ns)
packet size (bytes)
packet sender ip and port
