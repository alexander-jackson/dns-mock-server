use std::{
    net::{IpAddr, Ipv4Addr, SocketAddrV4},
    str::FromStr,
};

use tokio::net::UdpSocket;
use trust_dns_mock_server::Server;
use trust_dns_resolver::{
    config::{NameServerConfig, Protocol, ResolverConfig, ResolverOpts},
    proto::rr::LowerName,
    AsyncResolver,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn can_query_dns_records_from_the_server() -> Result<()> {
    let expected_addr = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

    let mut server = Server::default();
    server.add_records(
        LowerName::from_str("www.example.com.")?,
        vec![expected_addr],
    );

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
    let result = resolver.lookup_ip("www.example.com.").await?;

    let addrs: Vec<_> = result.into_iter().collect();

    assert_eq!(addrs.len(), 1);
    assert_eq!(addrs[0], expected_addr);

    Ok(())
}

#[tokio::test]
async fn unknown_names_return_requester_ip() -> Result<()> {
    let server = Server::default();

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
    let result = resolver.lookup_ip("www.example.com.").await?;

    let addrs: Vec<_> = result.into_iter().collect();

    assert_eq!(addrs.len(), 1);
    assert_eq!(addrs[0], IpAddr::V4(Ipv4Addr::LOCALHOST));

    Ok(())
}
