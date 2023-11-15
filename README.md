# `dns-mock-server`

Implementation of a DNS mock server for use in tests, based on
[`hickory-server`](https://github.com/hickory-dns/hickory-dns).

## Usage

The following example shows the basic usage for the library, where we create a
new server, add some records and then spawn it on a background task.

```rust
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

use dns_mock_server::Server;
use tokio::net::UdpSocket;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn check_something() -> Result<()> {
    let mut server = Server::default();

    let records = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    server.add_records("example.com", records)?

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    let socket = UdpSocket::bind(&addr).await?;
    let local_addr = socket.local_addr()?;

    tokio::spawn(async move {
        server.start(socket).await.unwrap();
    });

    // Point your DNS handling at `local_addr` and make requests

    Ok(())
}
```
