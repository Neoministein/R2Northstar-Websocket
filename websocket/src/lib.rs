use rrplug::prelude::*;
use rrplug::{
    bindings::convar::FCVAR_GAMEDLL,
    sq_return_bool, sq_return_notnull, sq_return_null,
    wrappers::{
        convars::ConVarRegister,
        northstar::EngineLoadType,
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
use rrplug::wrappers::convars::ConVarStruct;
use futures_util::stream::SplitSink;


struct WebSocketContainer
{
    write: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
   //read: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,

}

lazy_static! {
    static ref STREAM_MAP: Arc<Mutex<HashMap<String, WebSocketContainer>>> = Arc::new(Mutex::new(HashMap::new()));

    static ref RT: Runtime =  tokio::runtime::Runtime::new().unwrap();

    static ref LAST_MESSAGE:Arc<Mutex<HashMap<String, Vec<String>>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug)]
pub struct ExamplePlugin
{}


impl Plugin for ExamplePlugin {


    fn new() -> Self {
        Self {}
    }

    fn initialize(&mut self, plugin_data: &PluginData) {
        log::info!("yay logging on initialize :D");

        _ = plugin_data.register_sq_functions(info_example);
        _ = plugin_data.register_sq_functions(info_sq_connect_to_server);
        _ = plugin_data.register_sq_functions(info_sq_disconnect_from_server);
        _ = plugin_data.register_sq_functions(info_sq_write_message);
        _ = plugin_data.register_sq_functions(info_get_last_messages);


    }

    fn main(&self) {}


    fn on_engine_load(&self, engine: &EngineLoadType) {
        let engine = match engine {
            EngineLoadType::Engine(engine) => engine,
            EngineLoadType::EngineFailed => return,
            EngineLoadType::Server => return,
            EngineLoadType::Client => return,
        };

        let convar = ConVarStruct::try_new().unwrap();
        let register_info = ConVarRegister {
            callback: Some(basic_convar_changed_callback),
            ..ConVarRegister::mandatory(
                "basic_convar",
                "48",
                FCVAR_GAMEDLL.try_into().unwrap(),
                "basic_convar",
            )
        };

        convar.register(register_info).unwrap();

        _ = engine.register_concommand(
            "basic_command",
            basic_command_callback,
            "basic_command",
            FCVAR_GAMEDLL.try_into().unwrap(),
        );
    }

    type SaveType = squirrel::Save;
}

entry!(ExamplePlugin);


#[rrplug::concommand]
fn basic_command_callback(command: CCommandResult) {
    log::info!("running basic_command");
    log::info!("args: {:?}", command.args)
}

#[rrplug::convar]
fn basic_convar_changed_callback(convar: Option<ConVarStruct>, old_value: String, float_old_value: f32) {
    log::info!("old value: {}", float_old_value)
}

#[rrplug::sqfunction(VM=server,ExportName=BasicExample)]
 fn example(name: String) {
    log::info!("exmaple {name} ");

    sq_return_null!()
}
#[rrplug::sqfunction(VM=server,ExportName=NS_ConnectToWebsocket)]
fn sq_connect_to_server(socket_name: String, url: String, headers:String) -> bool {


    let mut was_success:bool = true;

    if !STREAM_MAP.lock().unwrap().contains_key(&socket_name)
    {
        was_success = RT.block_on(connect_to_server(socket_name,url,headers));

    }
    else {
        log::warn!("Tried to establish a second connection on {url}" );

    }

    sq_return_bool!(was_success, sqvm, sq_functions);
}

#[rrplug::sqfunction(VM=server,ExportName=NS_DisconnectFromWebsocket)]
fn sq_disconnect_from_server(socket_name: String)
{
    disconnect_from_server(socket_name);

    sq_return_null!();
}

#[rrplug::sqfunction(VM=server,ExportName=NS_WriteToWebsocket)]
fn sq_write_message(socket_name:String, message:String){

    RT.block_on(write_message(socket_name,message));

    sq_return_null!();
}

async fn write_message(socket_name : String, message: String){

    // Retrieve the map
    let map_lock = STREAM_MAP.lock().unwrap();

    // Get the WebSocketContainer from the map
    if let Some(container) = map_lock.get(&socket_name) {
        // Access the write field of the WebSocketContainer
        let mut write_mutex = container.write.lock().unwrap();
        let write = &mut *write_mutex;

        // Send the message
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(message.clone()))
            .await
            .expect("Failed to write the message");
        log::info!("Message send: [ {message} ]")

    } else {
        // Handle the case when the WebSocketContainer is not found
        log::error!("WebSocketContainer not found for socket_name: {}", socket_name);
    }
}

fn disconnect_from_server(socket_name: String)
{
    STREAM_MAP.lock().unwrap().remove(&socket_name);
}

async fn connect_to_server(socket_name: String, url_string: String, headers: String) -> bool {
    log::info!("Trying to connect ...");

    let header: Vec<&str> = headers.split("|#!#|").collect();



    let can_connect: bool;
    let mut request = url_string.clone().into_client_request().unwrap();

    let headers = request.headers_mut();

    for (header, value) in header.iter().step_by(2).zip(header.iter().skip(1).step_by(2)) {
        let header_name = HeaderName::from_str(header).unwrap();
        let header_value = HeaderValue::from_str(value).unwrap();

        log::info!("Used modified header: {header} and {value}");

        headers.insert(header_name, header_value);
    }

    let timeout_duration = Duration::from_secs(5); // Set the desired timeout duration

    let connect_result = timeout(timeout_duration, connect_async(request)).await;

    match connect_result {
        Ok(Ok(socket_stream)) => {
            log::info!("Connected to {url_string}");

            let (stream_stuff, _response) = socket_stream;

            let ( split_write, split_read) = stream_stuff.split();

            let new_container = WebSocketContainer {
                write: Arc::new(Mutex::new(split_write)),
                //read: Arc::new(Mutex::new(split_read)),
            };

            STREAM_MAP.lock().unwrap().insert(socket_name.clone(), new_container);
            LAST_MESSAGE.lock().unwrap().insert(socket_name.clone(), Vec::new());

            let socket_name_arc = Arc::new(socket_name.clone());

            tokio::spawn(async move {
                let socket_name_arc = socket_name_arc.clone();
                //let name = &socket_name.clone();
                let mut read_stream = split_read;

                while let Some(message) = read_stream.next().await {
                    let mut data = message.unwrap().into_data();
                    data.push(b'\n');
                    let s = String::from_utf8(data).expect("Found invalid UTF-8");

                    log::info!("From websocket {:?} : {:?}", socket_name_arc.clone() ,s.clone());

                    let lock = {
                        let socket_name_str = socket_name_arc.as_str().clone();
                        let last_message_map = LAST_MESSAGE.lock().unwrap();
                        let mut lock = last_message_map.get(socket_name_str).unwrap().clone();
                        lock.push(s.clone());
                        lock
                    };

                    let mut last_message_map = LAST_MESSAGE.lock().unwrap();
                    last_message_map.insert(socket_name_arc.as_str().clone().to_string(), lock);
                }
            });
            can_connect = true;
        }
        Ok(Err(e)) => {
            log::error!("Failed to connect : {:#?}", e);
            can_connect = false;
        }
        Err(_) => {
            log::error!("Connection timeout");
            can_connect = false;
        }
    }


    can_connect
}

#[rrplug::sqfunction(VM=server,ExportName=NS_GetLastMessages)]
fn get_last_messages(socket_name: String) -> Vec<String> {
    let mut last_message_map = LAST_MESSAGE.lock().unwrap();
    let  lock = last_message_map.get(&socket_name.clone()).unwrap().to_vec().clone();
    last_message_map.get_mut(&socket_name).unwrap().clear();

    push_sq_array(sqvm, sq_functions, lock);

    sq_return_notnull!()
}



// Cut stuff for now :( rip last 3 hours
//async fn check_if_socket_is_open(socket_name: &String) -> bool {
/*
    log::info!("Checking for socket connection! :D Fick dich neo");

    let mut buffer = [0; 10];

    let map = STREAM_MAP.lock().unwrap();




    //let  data = map.get(&socket_name).unwrap().get_ref()

    // Establish the connection

    let ws_container= map.get(&socket_name.clone()).unwrap();

    let mut write_mutex = ws_container.write.lock().unwrap();
    let write = &mut *write_mutex;

    let write_peek = write.peekable();

    let data = write_peek.peek();

    if data == None
    {
        return false;
    }

    return true;
}
 */


