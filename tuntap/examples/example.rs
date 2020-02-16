use tokio::stream::StreamExt;

use futures::sink::SinkExt;
use netif_tuntap::Native;

#[tokio::main]
pub async fn main() {
    let mut iface = Native::new().create_tun_async(0).expect("could not create TUN iface");
    let (mut tx, mut rx) = iface.pop_split_channels().expect("no split channels!");

    while let Some(packet) = rx.next().await {
        let packet = packet.unwrap();
        println!("received packet: {:x}", packet);
        tx.send(packet.freeze()).await.unwrap();
    }
}
