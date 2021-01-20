use log::{info, error};
use std::env;
use tonic::{transport::Server, Request, Response, Status};

// proto types
mod api;
use api::ping_pong_server::{PingPong, PingPongServer};
use api::{Ping, Pong};

#[derive(Debug, Default)]
pub struct PingPongService {}

#[tonic::async_trait]
impl PingPong for PingPongService {
    async fn send_ping(&self, request: Request<Ping>) -> Result<Response<Pong>, Status> {

        info!("received a request from {:?}", request.remote_addr());

        let pong = api::Pong {
            message: String::from ("pong")
        };

        Ok(Response::new(pong))
    }
}

#[derive(Debug, Clone, Default)]
struct Config {
    host : String,
    port : u32,
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    let config = Config {
        host: get_env_var ("HOST", Some (String::from ("127.0.0.1")))?,
        port: get_env_var ("PORT", Some (String::from ("3001")))?.parse::<u32>()?,
        log_level: get_env_var ("LOGGING_LEVEL", Some (String::from ("info")))?,
    };

    env::set_var("RUST_LOG", &config.log_level);
    env_logger::init();

    let pingpong = PingPongService::default ();

    let address = format!("{}:{}", &config.host, &config.port).parse().unwrap();
    info!("Server listening on {}", &address);

    Server::builder()
        .add_service(PingPongServer::new(pingpong))
        .serve(address)
        .await?;

    Ok(())
}

fn get_env_var (var : &str, default: Option<String> ) -> Result<String, anyhow::Error> {
    match env::var(var) {
        Ok (v) => Ok (v),
        Err (_) => {
            match default {
                None => {
                    error!("Missing ENV variable: {} not defined in environment", var);
                    panic! ("Missing ENV variable: {} not defined in environment", var);
                },
                Some (d) => Ok (d)
            }
        }
    }
}

pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}
