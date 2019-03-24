extern crate futures;
extern crate rand;
extern crate tokio;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::future::{lazy, loop_fn, Future, Loop};
use futures::stream::Stream;
use rand::Rng;
use tokio::net::{TcpListener, TcpStream};
use tokio::timer::{Delay, Interval};

type Counter = Arc<AtomicUsize>;
type TarpitState = (TcpStream, Counter);

fn main() {
    let addr = "0.0.0.0:22".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();
    let sock_stream = listener.incoming().map_err(|_| ());

    let connects = Arc::new(AtomicUsize::new(0));
    let disconnects = Arc::new(AtomicUsize::new(0));

    tokio::run(lazy(move || {
        stats_outputter(connects.clone(), disconnects.clone());

        sock_stream.for_each(move |client: TcpStream| {
            connects.fetch_add(1, Ordering::Relaxed);
            notify_conn(&client);
            tarpit((client, disconnects.clone()));
            Ok(())
        })
    }))
}

fn stats_outputter(connects: Counter, disconnects: Counter) {
    tokio::spawn(
        Interval::new_interval(Duration::from_secs(15))
            .map_err(|_| println!("Stats timer failed!"))
            .for_each(move |_| {
                let c = connects.load(Ordering::Relaxed);
                let d = disconnects.load(Ordering::Relaxed);
                Ok(println!("\nStats: {} total, {} current\n", c, c - d))
            }),
    );
}

fn tarpit(s: TarpitState) {
    let timer = Instant::now();
    tokio::spawn(loop_fn(s, move |(client, disconnects)| {
        slow_data(client).then(move |res| match res {
            Ok(client) => Ok(Loop::Continue((client, disconnects))),
            Err(raddr) => {
                disconnects.fetch_add(1, Ordering::Relaxed);
                notify_disconn(raddr, timer);
                Ok(Loop::Break(()))
            }
        })
    }));
}

fn slow_data(client: TcpStream) -> impl Future<Item = TcpStream, Error = SocketAddr> {
    let raddr = client.peer_addr().unwrap();
    Delay::new(Instant::now() + Duration::from_secs(10))
        .map_err(|_| std::io::ErrorKind::Other.into())
        .and_then(|_| tokio::io::write_all(client, random_text()))
        .map(|(client, _)| client)
        .map_err(move |_| raddr)
}

fn random_text() -> String {
    let mut rng = rand::thread_rng();
    let r: u64 = rng.gen();
    format!("{:x}\r\n", r)
}

fn notify_conn(client: &TcpStream) {
    let raddr = client.peer_addr().unwrap();
    println!("Connection from: {}", raddr);
}

fn notify_disconn(raddr: SocketAddr, timer: Instant) {
    let stop: Duration = Instant::now().duration_since(timer);
    println!("Connection broken: {} (lasted: {}s)", raddr, stop.as_secs());
}
