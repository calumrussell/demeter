use futures_util::{StreamExt, SinkExt, stream::SplitSink};
use std::future::Future;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, WebSocketStream, MaybeTlsStream, tungstenite::{Error, Message}};
use std::env;

//Because ETH JSON-RPC API is fixed this should be reusable code
#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: String,
    result: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcRequest{
    pub jsonrpc: String,
    pub method: String,
    pub id: String,
    pub params: Vec<MultipleTypes>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum MultipleTypes {
    Str(String),
    Bool(bool),
}

#[derive(Serialize, Deserialize, Debug)]
struct MultiStruct {
    pub key_1: i32,
    pub key_2: Vec<MultipleTypes>,
}

impl JsonRpcResponse {
    fn convert_result_from_hex(&self) -> i64 {
        i64::from_str_radix(&self.result.trim_start_matches("0x"), 16).unwrap()
    }
}


struct InfuraWS;

impl InfuraWS {

    fn get_ws_path() -> String {
        let token = match env::var_os("TOKEN") {
            Some(v) => v.into_string().unwrap(),
            None => panic!("$TOKENis not set")
        };
        return "wss://mainnet.infura.io/ws/v3/".to_owned() + &token;
    }

    fn ws_connect() -> impl Future<Output = Result<(WebSocketStream<MaybeTlsStream<TcpStream>>, tokio_tungstenite::tungstenite::handshake::client::Response), Error>> {
        let path = InfuraWS::get_ws_path();
        connect_async(path)
    }

    fn open_eth_blockByNumber(stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>) -> futures_util::sink::Send<'_, SplitSink<WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>, Message> {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: "eth_getBlockByNumber".to_string(),
            params: vec![MultipleTypes::Str("latest".to_string()), MultipleTypes::Bool(true)],
        };
        stream.send(tokio_tungstenite::tungstenite::Message::Text(serde_json::to_string(&req).unwrap()))
    } 

}

struct InfuraAPI;

impl InfuraAPI {
    fn get_path() -> String {
        let token = match env::var_os("TOKEN") {
            Some(v) => v.into_string().unwrap(),
            None => panic!("$TOKENis not set")
        };
        return "https://mainnet.infura.io/v3/".to_owned() + &token;
    }

    fn request_runner(method: &str, params: Vec<MultipleTypes>) -> impl Future<Output = Result<reqwest::Response, reqwest::Error>> {
        let req =  JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: "1".to_string(),
            method: method.to_string(),
            params,
        };
        let path = InfuraAPI::get_path();

        //Possible perf implications of doing this, I don't know what this does
        let client = reqwest::Client::new();
        client.post(path).json(&req).send()
    }
    async fn get_eth_blockByNumber() {
        let res = InfuraAPI::request_runner("eth_getBlockByNumber", vec![MultipleTypes::Str("latest".to_string()), MultipleTypes::Bool(true)]).await.unwrap();
        let text = res.text().await.unwrap();
        println!("{:?}", text);
    }

    async fn get_eth_blockNumber() -> i64 {
        let res = InfuraAPI::request_runner("eth_blockNumber", vec![]).await.unwrap();
        let text = res.text().await.unwrap();
        let json_response: JsonRpcResponse = serde_json::from_str(&text).unwrap();
        json_response.convert_result_from_hex()
    }

}

#[tokio::main]
async fn main() {
    let (stream, response) = InfuraWS::ws_connect().await.unwrap();
    let (mut write, read) = stream.split();

    InfuraWS::open_eth_blockByNumber(&mut write).await.unwrap();
    let read_future = read.for_each(|message| async move {
        println!("{:?}", message);
    });
    read_future.await;
}