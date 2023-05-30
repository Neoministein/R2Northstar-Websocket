# R2Northstar Websocket

This plugin is used to allow using Websockets in Squirrel.

## Installing the Plugin

1. Copy the `interstellar_websockets.dll` file into `%Titanfall2_InstallationFolder%/R2Northstar/plugins`

2. Copy the `Interstellar.WebsocketUtil` folder mod into `%Titanfall2_InstallationFolder%/R2Northstar/mods`

> It is not necessary to install the mod but, for ease of use we recommend to also use it, due to some limitations in the plugin v2 API.

## How to use the plugin/mod

Look at the util file at `Interstellar.WebsocketUtil\mod\scripts\vscripts\_WebsocketsUtil.gnut`. 
There you can find the functions to interact with the websocket. 

- PL_ is for methods defined in the plugin
- NS_ is for methods defined in the mod

> Note that an open websocket won't be closed on Squirrel VM destruction

## Developed by

- [Fuchsiano](https://github.com/Fuchsiano)
- [Neoministein](https://github.com/Neoministein)

