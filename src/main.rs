use std::sync::Arc;
use actix_web::{web, App, HttpServer, Responder};
use substrate_subxt::{
    ClientBuilder,
    DefaultNodeRuntime,
};
use url::Url;
use sp_rpc::number::NumberOrHex;
use sp_core::H256;
use parking_lot::Mutex;

type Client = substrate_subxt::Client::<DefaultNodeRuntime>;

struct Server {
    /// subxt client
    client: Client,
    /// tokio runtime,
    tokio_rt: Mutex<tokio::runtime::Runtime>,
}

impl Server {
    fn new() -> Self {
        let url: Url = "ws://127.0.0.1:9944".parse().expect("Fixed url, must parse; qed");
        let mut rt = tokio::runtime::Runtime::new().expect("Must be able to bootstrap Tokio Runtime");

        let client = loop {
            let client = ClientBuilder::<DefaultNodeRuntime>::new()
                .set_url(url.clone())
                .build();

            match rt.block_on(client) {
                Ok(client) => break client,
                Err(error) => println!("Failed to connect: {:?}", error),
            }
        };

        Server {
            client,
            tokio_rt: Mutex::new(rt),
        }
    }

    async fn hash(self: Arc<Self>, block: Option<u32>) -> Option<H256> {
        web::block(move || {
            let future = self.client.block_hash(block.map(NumberOrHex::Number));

            self.tokio_rt.lock().block_on(future)
        }).await.ok()?
    }
}

async fn block(server: web::Data<Server>, number: web::Path<u32>) -> impl Responder {
    format!("Hash: {:?}", server.into_inner().hash(Some(*number)).await)
}

async fn latest_block(server: web::Data<Server>) -> impl Responder {
    format!("Hash: {:?}", server.into_inner().hash(None).await)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let server = web::Data::new(Server::new());

    HttpServer::new(move || {
        App::new()
            .register_data(server.clone())
            .service(web::resource("/block").to(latest_block))
            .service(web::resource("/block/").to(latest_block))
            .service(web::resource("/block/{number}").to(block))
            .service(web::resource("/block/{number}/").to(block))
    })
    .bind("127.0.0.1:8080")?
    .start()
    .await
}