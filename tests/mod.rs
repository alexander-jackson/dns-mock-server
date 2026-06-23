use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};

use hickory_resolver::config::{ConnectionConfig, NameServerConfig, ResolverConfig};
use hickory_resolver::proto::op::ResponseCode;
use hickory_resolver::Resolver;
use hickory_server::net::runtime::TokioRuntimeProvider;
use hickory_server::net::{DnsError, NetError};
use tokio::net::UdpSocket;

use dns_mock_server::Server;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn can_query_dns_records_from_the_server() -> Result<()> {
    let expected_addr = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));

    let mut server = Server::default();
    server.add_records("www.example.com.", vec![expected_addr])?;

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    let socket = UdpSocket::bind(&addr).await?;

    let local_addr = socket.local_addr()?;

    tokio::spawn(async move {
        server.start(socket).await.unwrap();
    });

    let resolver = get_resolver(local_addr)?;

    let result = resolver.lookup_ip("www.example.com.").await?;
    let addrs: Vec<_> = result.iter().collect();

    assert_eq!(addrs.len(), 1);
    assert_eq!(addrs[0], expected_addr);

    Ok(())
}

#[tokio::test]
async fn unknown_names_return_errors() -> Result<()> {
    let server = Server::default();

    let addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0);
    let socket = UdpSocket::bind(&addr).await?;

    let local_addr = socket.local_addr()?;

    tokio::spawn(async move {
        server.start(socket).await.unwrap();
    });

    let resolver = get_resolver(local_addr)?;

    let Err(err) = resolver.lookup_ip("www.example.com.").await else {
        return Err("got successful response back".into());
    };

    let NetError::Dns(error) = err else {
        return Err("got unexpected error back".into());
    };

    let DnsError::NoRecordsFound(no_records) = error else {
        return Err("got unexpected error kind back".into());
    };

    assert_eq!(no_records.response_code, ResponseCode::NoError);

    Ok(())
}

fn get_resolver(addr: SocketAddr) -> Result<Resolver<TokioRuntimeProvider>> {
    let mut config = ResolverConfig::default();

    let connection_config = {
        let mut cfg = ConnectionConfig::udp();
        cfg.port = addr.port();

        cfg
    };

    let nameserver_config = NameServerConfig::new(addr.ip(), true, vec![connection_config]);
    config.add_name_server(nameserver_config);

    let provider = TokioRuntimeProvider::default();
    let resolver = Resolver::builder_with_config(config, provider).build()?;

    Ok(resolver)
}
