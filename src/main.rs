extern crate futures;
extern crate rand;
extern crate tokio;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use futures::future::{lazy, loop_fn, Future, Loop};
use futures::stream::Stream;
use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio::timer::{Delay, Interval};

const TRAP_TIME: u64 = 10;
const STATS_INTERVAL: u64 = 15;

static CON: AtomicUsize = AtomicUsize::new(0);
static DIS: AtomicUsize = AtomicUsize::new(0);

fn main() {
    let addr = "0.0.0.0:22".parse::<SocketAddr>().unwrap();
    let sock_stream = TcpListener::bind(&addr).unwrap().incoming().map_err(|_| ());

    tokio::run(lazy(move || {
        stats_outputter();

        sock_stream.for_each(move |client: TcpStream| {
            tarpit(client);
            Ok(())
        })
    }))
}

fn stats_outputter() {
    tokio::spawn(
        Interval::new_interval(Duration::from_secs(STATS_INTERVAL))
            .map_err(|_| println!("Stats timer failed!"))
            .for_each(|_| Ok(notify_stats())),
    );
}

fn tarpit(client: TcpStream) {
    let timer = Instant::now();
    notify_conn(&client);
    tokio::spawn(loop_fn(client, move |client| {
        slow_data(client).then(move |res| match res {
            Ok(client) => Ok(Loop::Continue(client)),
            Err(raddr) => {
                notify_disconn(raddr, timer);
                Ok(Loop::Break(()))
            }
        })
    }));
}

fn slow_data(client: TcpStream) -> impl Future<Item = TcpStream, Error = SocketAddr> {
    let raddr = client.peer_addr().unwrap();
    Delay::new(Instant::now() + Duration::from_secs(TRAP_TIME))
        .map_err(|_| std::io::ErrorKind::Other.into())
        .and_then(|_| tokio::io::write_all(client, random_text()))
        .map(|(client, _)| client)
        .map_err(move |_| raddr)
}

fn notify_conn(client: &TcpStream) {
    CON.fetch_add(1, Ordering::Relaxed);
    let raddr = client.peer_addr().unwrap();
    println!("Connection from: {}", raddr);
}

fn notify_disconn(raddr: SocketAddr, timer: Instant) {
    DIS.fetch_add(1, Ordering::Relaxed);
    let stop: Duration = Instant::now().duration_since(timer);
    println!("Connection broken: {} (lasted: {}s)", raddr, stop.as_secs());
}

fn notify_stats() {
    let c = CON.load(Ordering::Relaxed);
    let d = DIS.load(Ordering::Relaxed);
    println!("\nStats: {} total, {} current\n", c, c - d)
}

fn random_text() -> String {
    let mut rng = rand::thread_rng();
    let r: u64 = rng.gen();
    format!("{:x}\r\n", r)
}
