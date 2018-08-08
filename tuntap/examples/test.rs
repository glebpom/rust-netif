extern crate futures;
extern crate tokio;
extern crate netif_tuntap;

use futures::{Future, Sink, Stream};
use std::io;
use std::io::Read;
use std::time::{Duration, Instant};
use tokio::runtime::current_thread::Runtime;
use tokio::timer::Delay;
use netif_tuntap::Native;

fn main() {
    // let lcstr_driver = tuntap::TunTapOsx::new(None, Some(tuntap::DevPath {
    //     dev_name: "lcstrtap".to_owned(),
    //     max_num: 64,
    // }));
    // println!("LCSTR TAP: Probe = {}", lcstr_driver.probe_tap());
    // lcstr_driver.create_tap().unwrap();

    let mut runtime = Runtime::new().unwrap();
    // let driver = tuntap::TunTapOsx::default();
    // let mut tap = driver.create_tap(Some(52)).unwrap();

    let native = Native::new();
    let mut tap = native.create_tun(0).unwrap();

    println!("info = {:?}", tap.info().unwrap());

    let (tx, rx) = tap.pop_channels_spawn_threads(16).unwrap().unwrap();

    runtime.block_on(rx.for_each(|packet| {
        println!("1 Received packet {:x?}", packet);
        Ok(())
    }));

    // let native = tuntap::Native::new();
    // // let mut tun = native.create_tun().unwrap();

    // let mut tun = native.create_tun_async(0).unwrap();
    // // let mut tun = native.create_tun_async().unwrap();

    // let f1 = tun.pop_queue().unwrap().for_each(|packet| {
    //     println!("1 Received packet {:x?}", packet);
    //     Delay::new(Instant::now() + Duration::from_millis(10)).map_err(|_| io::Error::new(io::ErrorKind::Other, ""))
    // }).then(|_| Ok::<(),()>(()));
    // // let f2 = tun.pop_queue().unwrap().for_each(|packet| {
    // //     println!("2 Received packet {:x?}", packet);
    // //     Delay::new(Instant::now() + Duration::from_millis(100)).map_err(|_| io::Error::new(io::ErrorKind::Other, ""))
    // // }).then(|_| Ok::<(),()>(()));
    // // let f3 = tun.pop_queue().unwrap().for_each(|packet| {
    // //     println!("3 Received packet {:x?}", packet);
    // //     Delay::new(Instant::now() + Duration::from_millis(100)).map_err(|_| io::Error::new(io::ErrorKind::Other, ""))
    // // }).then(|_| Ok::<(),()>(()));
    // // let f4 = tun.pop_queue().unwrap().for_each(|packet| {
    // //     println!("4 Received packet {:x?}", packet);
    // //     Delay::new(Instant::now() + Duration::from_millis(100)).map_err(|_| io::Error::new(io::ErrorKind::Other, ""))
    // // }).then(|_| Ok::<(),()>(()));

    // // runtime.spawn(f4);
    // // runtime.spawn(f3);
    // // runtime.spawn(f2);
    // runtime.block_on(f1).unwrap();
}
