use async_trait::async_trait;
use tokio::net::UdpSocket;
use trust_dns_server::{
    server::{Request, RequestHandler, ResponseHandler, ResponseInfo},
    ServerFuture,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Server;

impl Server {
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
        _request: &Request,
        _response: R,
    ) -> ResponseInfo {
        todo!()
    }
}
