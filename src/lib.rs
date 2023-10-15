use std::{collections::HashMap, net::IpAddr};

use async_trait::async_trait;
use tokio::net::UdpSocket;
use trust_dns_server::proto::{
    op::Header,
    rr::{
        rdata::{A, AAAA},
        LowerName, RData, Record,
    },
};
use trust_dns_server::{
    authority::MessageResponseBuilder,
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    ServerFuture,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, Default)]
pub struct Server {
    store: HashMap<LowerName, Vec<IpAddr>>,
}

impl Server {
    pub fn add_records(&mut self, name: LowerName, records: Vec<IpAddr>) {
        self.store.insert(name, records);
    }

    pub async fn start(self, socket: UdpSocket) -> Result<()> {
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

        if let Some(entries) = self.store.get(name) {
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
        } else {
            let rdata = match request.src().ip() {
                IpAddr::V4(ipv4) => RData::A(A::from(ipv4)),
                IpAddr::V6(ipv6) => RData::AAAA(AAAA::from(ipv6)),
            };

            let records = vec![Record::from_rdata(request.query().name().into(), 60, rdata)];
            let response = builder.build(header, records.iter(), &[], &[], &[]);

            response_handler.send_response(response).await.unwrap()
        }
    }
}
