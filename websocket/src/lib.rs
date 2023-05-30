use rrplug::prelude::*;
use rrplug::{
    sq_return_bool, sq_return_notnull, sq_return_null,
    wrappers::{
        squirrel::push_sq_array,
    },
};

use std::{
    collections::HashMap,
    time::Duration,
    sync::Arc,
    str::FromStr,
};

use tokio::{
    time::timeout,
    net::TcpStream,
};

use tokio_tungstenite::{
    WebSocketStream, MaybeTlsStream,
    tungstenite::{
        client::IntoClientRequest,
        http::{HeaderName, HeaderValue},
        Message,
    },
    connect_async,
};

use futures_util::{
    stream::StreamExt,
    sink::SinkExt,
};
use std::sync::Mutex;
use lazy_static::lazy_static;
use tokio::runtime::Runtime;
use futures_util::stream::SplitSink;


struct WebSocketContainer
{
    write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
}

lazy_static! {
    static ref STREAM_MAP: Arc<Mutex<HashMap<String, WebSocketContainer>>> = Arc::new(Mutex::new(HashMap::new()));

    static ref RT: Runtime =  tokio::runtime::Runtime::new().unwrap();

    static ref LAST_MESSAGE:Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug)]
pub struct WebsocketPlugin
{}


impl Plugin for WebsocketPlugin {
    type SaveType = squirrel::Save;

    fn new() -> Self {
        Self {}
    }

    fn initialize(&mut self, plugin_data: &PluginData) {
        _ = plugin_data.register_sq_functions(info_sq_connect_to_server);
        _ = plugin_data.register_sq_functions(info_sq_disconnect_from_server);
        _ = plugin_data.register_sq_functions(info_sq_write_message);
        _ = plugin_data.register_sq_functions(info_get_last_messages);
    }

    fn main(&self) {}
}

entry!(WebsocketPlugin);

#[rrplug::sqfunction(VM=server,ExportName=PL_ConnectToWebsocket)]
fn sq_connect_to_server(socket_name: String, url: String, headers:String, connection_time_out: i32) -> bool {
    log::info!("Trying to establish websocket connection [{socket_name}] to [{url}]" );

    if STREAM_MAP.lock().unwrap().contains_key(&socket_name)
    {
        log::warn!("There is still a open websocket connection under [{socket_name}] closing websocket." );
        disconnect_from_server(&socket_name);
    }

    let was_success = RT.block_on(connect_to_server(socket_name,url,headers, connection_time_out as u64));

    sq_return_bool!(was_success, sqvm, sq_functions);
}

#[rrplug::sqfunction(VM=server,ExportName=PL_DisconnectFromWebsocket)]
fn sq_disconnect_from_server(socket_name: String) {
    log::info!("Disconnecting websocket client [{socket_name}]");

    disconnect_from_server(&socket_name);

    sq_return_null!();
}

#[rrplug::sqfunction(VM=server,ExportName=PL_WriteToWebsocket)]
fn sq_write_message(socket_name:String, message:String) {
    log::trace!("Writing to websocket [{socket_name}] message [{message}]");

    RT.block_on(write_message(socket_name,message));

    sq_return_null!();
}

#[rrplug::sqfunction(VM=server,ExportName=PL_ReadFromWebsocket)]
fn get_last_messages(socket_name: String) -> Vec<String> {
    log::trace!("Trying to read from the websocket [{socket_name}] buffer");

    let mut last_message_map = LAST_MESSAGE.lock().unwrap();
    let  lock = last_message_map.get(&socket_name.clone()).unwrap().to_vec().clone();
    last_message_map.get_mut(&socket_name).unwrap().clear();

    push_sq_array(sqvm, sq_functions, lock);

    sq_return_notnull!()
}

async fn write_message(socket_name : String, message: String) {

    // Retrieve the map
    let map_lock = STREAM_MAP.lock().unwrap();

    // Get the WebSocketContainer from the map
    if let Some(container) = map_lock.get(&socket_name) {
        // Access the write field of the WebSocketContainer
        let mut write_mutex = container.write.lock().unwrap();
        let write = &mut *write_mutex;

        // Send the message
        write
            .send(Message::Text(message.clone()))
            .await
            .expect("Failed to write the message");
        log::trace!("Message for [{socket_name}] was sent successful [{message}]")

    } else {
        // Handle the case when the WebSocketContainer is not found
        log::warn!("There is no established connection for [{socket_name}]");
    }
}

fn disconnect_from_server(socket_name: &String)
{
    RT.block_on(STREAM_MAP.lock().unwrap().get(socket_name).unwrap().write.lock().unwrap().close()).expect("issue closing websocket");
    STREAM_MAP.lock().unwrap().remove(socket_name);
}

async fn connect_to_server(socket_name: String, url_string: String, headers: String, connection_time_out: u64) -> bool {
    log::debug!("Trying to establish websocket connection [{socket_name}]..." );

    let header: Vec<&str> = headers.split("|#!#|").collect();

    let can_connect: bool;

    log::debug!("Config: [{socket_name}] url = [{url_string}]");
    let mut request = url_string.clone().into_client_request().unwrap();

    let headers = request.headers_mut();

    log::debug!("Config: [{socket_name}] parsing headers...");
    for (header, value) in header.iter().step_by(2).zip(header.iter().skip(1).step_by(2)) {
        let header_name = HeaderName::from_str(header).unwrap();
        let header_value = HeaderValue::from_str(value).unwrap();

        log::debug!("Config: [{socket_name}] Adding header [{header}] value: [{value}]");

        headers.insert(header_name, header_value);
    }

    log::debug!("Config: [{socket_name}] connection timeout [{}s]", connection_time_out);
    let timeout_duration = Duration::from_secs(connection_time_out); // Set the desired timeout duration

    let connect_result = timeout(timeout_duration, connect_async(request)).await;

    match connect_result {
        Ok(Ok(socket_stream)) => {
            log::info!("Connection successful for [{url_string}]");

            let (stream_stuff, _response) = socket_stream;

            let ( split_write, split_read) = stream_stuff.split();

            let new_container = WebSocketContainer {
                write: Arc::new(Mutex::new(split_write)),
            };

            STREAM_MAP.lock().unwrap().insert(socket_name.clone(), new_container);
            LAST_MESSAGE.lock().unwrap().insert(socket_name.clone(), Vec::new());

            let socket_name_arc = Arc::new(socket_name.clone());

            tokio::spawn(async move {
                log::info!("Spinning up listening thread for [{socket_name}]");

                let socket_name_arc = socket_name_arc.clone();

                let mut read_stream = split_read;

                while let Some(result) = read_stream.next().await {
                    match result {
                        Err(_) => log::warn!("Websocket [{socket_name}] closed unexpectedly"),
                        Ok(message) => {
                            let data = message.into_data();
                            let s = String::from_utf8(data).expect("Websocket provided invalid UTF-8");

                            log::trace!("Received message from Websocket [{:?}] message [{:?}]", socket_name_arc.clone() ,s.clone());

                            let lock = {
                                let socket_name_str = socket_name_arc.as_str().clone();
                                let last_message_map = LAST_MESSAGE.lock().unwrap();
                                let mut lock = last_message_map.get(socket_name_str).unwrap().clone();
                                lock.push(s.clone());
                                lock
                            };

                            let mut last_message_map = LAST_MESSAGE.lock().unwrap();
                            last_message_map.insert(socket_name_arc.as_str().clone().to_string(), lock);
                        },
                    }
                }
            });
            can_connect = true;
        }
        Ok(Err(e)) => {
            log::error!("Failed to connect to {socket_name} reason: {:#?}", e);
            can_connect = false;
        }
        Err(_) => {
            log::error!("Timeout was reached while trying to connect to [{socket_name}]");
            can_connect = false;
        }
    }

    can_connect
}

impl Drop for WebsocketPlugin {

    fn drop(&mut self) {
        for (key, _) in &*STREAM_MAP.lock().unwrap() {
            disconnect_from_server(key)
        }
    }
}