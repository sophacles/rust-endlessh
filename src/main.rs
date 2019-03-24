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

fn random_text() -> String {
    let mut rng = rand::thread_rng();
    let r: u64 = rng.gen();
    let out = format!("{:x}\r\n", r);
    out.into()
}

fn main() {
    let addr = "0.0.0.0:22".parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).unwrap();

    let connects = Arc::new(AtomicUsize::new(0));
    let disconnects = Arc::new(AtomicUsize::new(0));

    let connects_timer = connects.clone();
    let disconnects_timer = disconnects.clone();
    tokio::run(lazy(move || {
        tokio::spawn(
            Interval::new_interval(Duration::from_secs(15))
                .map_err(|_| println!("Stats timer failed!"))
                .for_each(move |_| {
                    let c = connects_timer.load(Ordering::Relaxed);
                    let d = disconnects_timer.load(Ordering::Relaxed);
                    println!("\nStats: {} total, {} current\n", c, c - d);
                    Ok(())
                }),
        );
        listener
            .incoming()
            .map_err(|_| ())
            .for_each(move |client: TcpStream| {
                let raddr = client.peer_addr().unwrap().clone();
                println!("Connection from: {}", raddr);

                connects.fetch_add(1, Ordering::Relaxed);
                let disconnects_tarpit = disconnects.clone();
                let timer = Instant::now();

                tokio::spawn(loop_fn(
                    (client, disconnects_tarpit),
                    move |(client, disconnects)| {
                        Delay::new(Instant::now() + Duration::from_secs(10))
                            .map_err(|_| std::io::ErrorKind::Other.into())
                            .and_then(|_| tokio::io::write_all(client, random_text()))
                            .then(move |res| match res {
                                Ok((client, _)) => Ok(Loop::Continue((client, disconnects))),
                                Err(_) => {
                                    let stop: Duration = Instant::now().duration_since(timer);
                                    disconnects.fetch_add(1, Ordering::Relaxed);
                                    println!(
                                        "Connection broken: {} (lasted: {}s)",
                                        raddr,
                                        stop.as_secs()
                                    );
                                    Ok(Loop::Break(()))
                                }
                            })
                    },
                ));
                Ok(())
            })
    }))
}
