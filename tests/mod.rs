use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};

use hickory_proto::xfer::Protocol;
use hickory_resolver::config::{NameServerConfig, ResolverConfig};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::op::ResponseCode;
use hickory_resolver::ResolveErrorKind;
use hickory_resolver::Resolver;
use hickory_server::proto::ProtoErrorKind;
use tokio::net::UdpSocket;

use dns_mock_server::{Response, Server};

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

    let mut config = ResolverConfig::new();
    let nameserver_config = NameServerConfig::new(local_addr, Protocol::Udp);
    config.add_name_server(nameserver_config);

    let resolver =
        Resolver::builder_with_config(config, TokioConnectionProvider::default()).build();
    let result = resolver.lookup_ip("www.example.com.").await?;

    let addrs: Vec<_> = result.into_iter().collect();

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

    let mut config = ResolverConfig::new();
    let nameserver_config = NameServerConfig::new(local_addr, Protocol::Udp);
    config.add_name_server(nameserver_config);

    let resolver =
        Resolver::builder_with_config(config, TokioConnectionProvider::default()).build();

    let Err(err) = resolver.lookup_ip("www.example.com.").await else {
        return Err("got successful response back".into());
    };

    let ResolveErrorKind::Proto(proto_error) = err.kind() else {
        return Err("got unexpected error kind back".into());
    };

    let ProtoErrorKind::NoRecordsFound { response_code, .. } = proto_error.kind() else {
        return Err("got unexpected proto error kind back".into());
    };

    assert_eq!(*response_code, ResponseCode::ServFail);

    Ok(())
}

#[tokio::test]
async fn can_query_desired_response_from_the_server() -> Result<()> {
    let mut server = Server::default();
    server.add_response("www.example.com.", Response::Code(ResponseCode::NXDomain))?;

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

    match result.unwrap_err().kind() {
        ResolveErrorKind::NoRecordsFound { response_code, .. } => {
            assert_eq!(*response_code, ResponseCode::NXDomain)
        }
        _ => panic!("wrong response code"),
    };

    Ok(())
}
