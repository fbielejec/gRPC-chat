use futures::{Stream, StreamExt};
use log::{debug, info, error, warn};
use redis::{AsyncCommands, RedisResult};
use std::env;
use std::pin::Pin;
use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};

// TODO
// - server reflection

// proto types
mod api;
use api::ping_pong_server::{PingPong, PingPongServer};
use api::chat_server::{Chat, ChatServer};
use api::{Ping, Pong, ChatMessage};

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
    redis_node: String,
}

#[derive(Debug, Default)]
pub struct ChatService {
    config: Config
}

#[tonic::async_trait]
impl Chat for ChatService {

    type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatMessage, Status>> + Send + Sync + 'static>>;

    async fn chat(&self,
                  request: Request<tonic::Streaming<ChatMessage>>)
                  -> Result<Response<Self::ChatStream>, tonic::Status> {

        let Config { redis_node, .. } = &self.config;

        let user_id = String::from (request.metadata ().get ("from").unwrap ().to_str ().unwrap ());

        info!("user connected with id: {:#?}", &user_id);

        let from = user_id.clone ();
        let (tx, rx)
        // : (tokio::sync::mpsc::Sender<ChatMessage>, tokio::sync::mpsc::Receiver<ChatMessage>)
            = mpsc::channel(4);

        let message = ChatMessage {to: from.clone (),
                                   message: format! ("You are connected with id: {}", from.clone ())};
        tx.send ( Ok (message) ).await.unwrap();
        info!("sent on-connect message");

        let client = redis::Client::open(redis_node.to_owned ()).unwrap();
        let mut redis_pub : redis::aio::Connection = client.get_async_connection().await.unwrap();
        // TODO: Arc Mutex
        let mut redis_sub = client.get_async_connection().await.unwrap().into_pubsub();

        redis_sub.subscribe(&user_id).await.unwrap();

        tokio::spawn(async move {
            let mut sub_stream = redis_sub.on_message();
            let mut optional = sub_stream.next().await;
            while let Some(ref msg) = optional {

                info!("Received message {:#?} on channel: {:#?}", &msg, &from);

                let payload : String = msg.get_payload ().unwrap ();
                let chat_message = ChatMessage {to: from.clone (),
                                                message: payload};

                // handle failed send
                match tx.send ( Ok (chat_message) ).await {
                    Ok (_) => {
                        debug!("Succesfully sent message to gRPC client: {}", &from);
                        optional = sub_stream.next().await;
                    },
                    Err (err) => {
                        warn!("Error : {} when sending message to gRPC client : {}", &err, &from);
                        optional = None;
                    }
                }
            }
        });

        let mut request_stream = request.into_inner();
        tokio::spawn(async move {
            let mut optional = request_stream.next().await;
            // NOTE: equivalent to: Some(chat_message) = &optional
            while let Some(ref chat_message) = optional {

                info!("Received message {:#?} from gRPC client: {:#?}", &chat_message, &user_id);

                // handle disconnected client
                match chat_message {
                    Ok (chat_message) => {
                        let to = &chat_message.to;
                        let message = &chat_message.message;
                        let _ : RedisResult<String> = redis_pub.publish(to, message).await;
                        debug!("Succesfully published message on channel: {}", &to);
                        optional = request_stream.next().await;
                    },
                    Err (_) => {
                        warn!("client disconnected: {}", &user_id);
                        // redis_sub.unsubscribe (&user_id);
                        optional = None;
                    }
                }
            }
        });

        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx))))
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {

    let config = Config {
        host: get_env_var ("HOST", Some (String::from ("127.0.0.1")))?,
        port: get_env_var ("PORT", Some (String::from ("3001")))?.parse::<u32>()?,
        log_level: get_env_var ("LOGGING_LEVEL", Some (String::from ("info")))?,
        redis_node: get_env_var ("REDIS_NODE", Some (String::from ("redis://127.0.0.1:6379")))?,
    };

    env::set_var("RUST_LOG", &config.log_level);
    env_logger::init();

    let pingpong = PingPongService::default ();

    let chat = ChatService {
        config: config.clone ()
    };

    let address = format!("{}:{}", &config.host, &config.port).parse().unwrap();
    info!("Server listening on {}", &address);

    Server::builder()
        .add_service(PingPongServer::new(pingpong))
        .add_service(ChatServer::new(chat))
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
