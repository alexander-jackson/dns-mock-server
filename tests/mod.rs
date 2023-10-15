use std::net::{Ipv4Addr, SocketAddrV4};

use tokio::net::UdpSocket;
use trust_dns_mock_server::Server;
use trust_dns_resolver::{
    config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
    AsyncResolver,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn can_query_dns_records_from_the_server() -> Result<()> {
    let server = Server;

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    let socket = UdpSocket::bind(&addr).await?;

    let local_addr = socket.local_addr()?;

    tokio::spawn(async move {
        server.start(socket).await.unwrap();
    });

    let mut config = ResolverConfig::new();
    let nameserver_config = NameServerConfig::new(local_addr, Protocol::Udp);
    config.add_name_server(nameserver_config);

    let resolver = AsyncResolver::tokio(config, ResolverOpts::default());
    let result = resolver.lookup_ip("www.example.com.").await;

    assert!(result.is_ok());

    Ok(())
}
