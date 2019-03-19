use neqo_crypto::init_db;
use neqo_transport::{Connection, Datagram};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "neqo-client", about = "A basic QUIC client.")]
struct Args {
    host: String,
    port: u16,
    #[structopt(short = "d", long, default_value = "./db", parse(from_os_str))]
    db: PathBuf,

    #[structopt(short = "4", long)]
    /// Restrict to IPv4.
    ipv4: bool,
    #[structopt(short = "6", long)]
    /// Restrict to IPv6.
    ipv6: bool,
}

impl Args {
    fn addr(&self) -> SocketAddr {
        self.to_socket_addrs()
            .expect("Remote address error")
            .next()
            .expect("No remote addresses")
    }

    fn bind(&self) -> SocketAddr {
        match self.addr() {
            SocketAddr::V4(..) => SocketAddr::new(IpAddr::V4(Ipv4Addr::from([0; 4])), 0),
            SocketAddr::V6(..) => SocketAddr::new(IpAddr::V6(Ipv6Addr::from([0; 16])), 0),
        }
    }
}

impl ToSocketAddrs for Args {
    type Iter = ::std::option::IntoIter<SocketAddr>;
    fn to_socket_addrs(&self) -> ::std::io::Result<Self::Iter> {
        (
            IpAddr::from_str(self.host.as_str()).expect("Invalid address"),
            self.port,
        )
            .to_socket_addrs()
    }
}

fn main() {
    let args = Args::from_args();
    init_db(args.db.clone());

    let socket = UdpSocket::bind(args.bind()).expect("Unable to bind UDP socket");
    socket.connect(&args).expect("Unable to connect UDP socket");

    let local_addr = socket.local_addr().expect("Socket local address not bound");
    let remote_addr = args.addr();
    let mut client = Connection::new_client(args.host.as_str(), local_addr, remote_addr);

    let buf = &mut [0u8; 2048];
    let mut in_dgrams = Vec::new();
    loop {
        let out_dgrams = client
            .process(in_dgrams.drain(..))
            .expect("Error processing input");

        for d in out_dgrams {
            let sent = socket.send(&d[..]).expect("Error sending datagram");
            if sent != d.len() {
                eprintln!("Unable to send all {} bytes of datagram", d.len());
            }
        }

        let sz = socket.recv(&mut buf[..]).expect("UDP error");
        if sz == buf.len() {
            eprintln!("Received more than {} bytes", buf.len());
            continue;
        }
        if sz > 0 {
            in_dgrams.push(Datagram::new(remote_addr, local_addr, &buf[..sz]));
        }
    }
}