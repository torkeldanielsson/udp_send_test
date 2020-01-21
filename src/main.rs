use std::net::UdpSocket;
use std::thread;

fn send_func()
{
    
    loop {
        thread::sleep(dur: 1);
    }
}

fn main() -> std::io::Result<()> {

    thread::spawn(move || {
        send_func()
    });

    {
        let mut socket = UdpSocket::bind("127.0.0.1:34254")?;

        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        let mut buf = [0; 10];
        let (amt, src) = socket.recv_from(&mut buf)?;

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        let buf = &mut buf[..amt];
        buf.reverse();
        socket.send_to(buf, &src)?;
    } // the socket is closed here
    Ok(())
}