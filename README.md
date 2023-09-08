# R2Northstar Websocket

This plugin is used to allow using Websockets in Squirrel.

## Installing the Plugin

1. Download the latest release of the plugin from the [release tab](https://github.com/Neoministein/R2Northstar-Websocket/releases)

2. Copy the `interstellar_websockets.dll` file into `%Titanfall2_InstallationFolder%/R2Northstar/plugins`

3. Copy the `Interstellar.WebsocketUtil` folder mod into `%Titanfall2_InstallationFolder%/R2Northstar/mods`

> It is not necessary to install the mod but, for ease of use we recommend to also use it, due to some limitations in the plugin v2 API.

> When using wine this plugin will only work when with version 7.13 and above.

## How to use the plugin/mod

Look at the util file at `Interstellar.WebsocketUtil\mod\scripts\vscripts\_WebsocketsUtil.gnut`. 
There you can find the functions to interact with the websocket. 

- PL_ is for methods defined in the plugin
- NS_ is for methods defined in the mod

> Note that an open websocket won't be closed on Squirrel VM destruction
## Functions
- `PL_ConnectToWebsocket`
	- Used to connect to socket.
	- `socket_name`:  the name the socket is referred by.
	- `url`: url of the websocket.
	- `header`: a singel string from a tabel created with the `HeaderConverter` util function. 
	- `returning`: successfully connected true/false.
	
- `PL_DisconnectFromWebsocket`: 
	- Used to disconnect from the socket 

- `PL_WriteToWebsocket`
	-  Used to send a message to the webserver
	- `socket_name`: name of the connected websocket. 
	-  `message`: message for the server.

- `PL_ReadFromWebsocket`
	- Get all messages from connection since last `PL_ReadFromWebsocket` call.
	- `socket_name`: name of the connected websocket.
	- `returning`: string array containing all messages in chronological order.
- `PL_GetOpenWebsockets`
  - Get all open websocket connection names.
  - `returning`: string array containing open websocket connection names.
 
## Developed by

- [Fuchsiano](https://github.com/Fuchsiano)
- [Neoministein](https://github.com/Neoministein)

