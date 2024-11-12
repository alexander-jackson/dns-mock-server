//! Implementation of a DNS server intended for use in tests.
//!
//! This allows you to run a proper DNS server while setting up records to be mapped to specific IP
//! addresses. Your test code can then target the locally bound server and make normal DNS
//! requests.

use std::collections::HashMap;
use std::net::IpAddr;
use std::str::FromStr;

use async_trait::async_trait;
use hickory_server::authority::MessageResponseBuilder;
use hickory_server::proto::error::ProtoError;
use hickory_server::proto::op::Header;
use hickory_server::proto::op::ResponseCode;
use hickory_server::proto::rr::rdata::{A, AAAA};
use hickory_server::proto::rr::{LowerName, RData, Record};
use hickory_server::server::{
    Request, RequestHandler, ResponseHandler, ResponseInfo, ServerFuture,
};
use tokio::net::UdpSocket;

/// A simple mock server for DNS requests.
///
/// The intended usage is to create a new instance using [`Server::default()`] and add some record
/// mappings to it. You can then bind a [`UdpSocket`] and start the server with [`Server::start()`]
/// in a background task before making requests on the main thread.
#[derive(Clone, Debug, Default)]
pub struct Server {
    store: HashMap<LowerName, Response>,
}

#[derive(Clone, Debug)]
pub enum Response {
    Ok(Vec<IpAddr>),
    Code(ResponseCode),
}

impl Server {
    /// Adds a mapping from a DNS record to some IP addresses.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::net::{IpAddr, Ipv4Addr};
    /// # use dns_mock_server::Server;
    /// let mut server = Server::default();
    /// let records = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    ///
    /// server.add_records("example.com", records).expect("Invalid hostname");
    /// ```
    pub fn add_records(&mut self, name: &str, records: Vec<IpAddr>) -> Result<(), ProtoError> {
        self.add_response(name, Response::Ok(records))
    }

    /// Adds a mapping from a DNS record to some general response (allows to use any response code)
    ///
    /// # Example
    ///
    /// ```
    /// # use hickory_server::proto::op::ResponseCode;
    /// # use dns_mock_server::Server;
    /// let mut server = Server::default();
    /// let response = Response::Code(ResponseCode::NXDomain);
    ///
    /// server.add_response("example.com", response).expect("Invalid hostname");
    /// ```
    pub fn add_response(&mut self, name: &str, response: Response) -> Result<(), ProtoError> {
        let name = LowerName::from_str(name)?;

        self.store.insert(name, response);

        Ok(())
    }

    /// Starts the mock server on the given [`UdpSocket`].
    ///
    /// This should be run in a background task using a method such as [`tokio::spawn`].
    pub async fn start(self, socket: UdpSocket) -> Result<(), ProtoError> {
        let mut server = ServerFuture::new(self);

        server.register_socket(socket);
        server.block_until_done().await?;

        Ok(())
    }
}

#[async_trait]
impl RequestHandler for Server {
    async fn handle_request<R: ResponseHandler>(
        &self,
        request: &Request,
        mut response_handler: R,
    ) -> ResponseInfo {
        let builder = MessageResponseBuilder::from_message_request(request);

        let mut header = Header::response_from_request(request.header());
        header.set_authoritative(true);

        let name = request.query().name();

        match self.store.get(name) {
            Some(Response::Ok(entries)) => {
                let records: Vec<_> = entries
                    .iter()
                    .map(|entry| match entry {
                        IpAddr::V4(ipv4) => RData::A(A::from(*ipv4)),
                        IpAddr::V6(ipv6) => RData::AAAA(AAAA::from(*ipv6)),
                    })
                    .map(|rdata| Record::from_rdata(name.into(), 60, rdata))
                    .collect();

                let response = builder.build(header, records.iter(), &[], &[], &[]);
                response_handler.send_response(response).await.unwrap()
            }
            Some(Response::Code(code)) => {
                header.set_response_code(*code);

                let response = builder.build_no_records(header);
                response_handler.send_response(response).await.unwrap()
            }
            _ => {
                header.set_response_code(ResponseCode::ServFail);

                let response = builder.build_no_records(header);
                response_handler.send_response(response).await.unwrap()
            }
        }
    }
}
