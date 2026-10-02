## ⚠️ Work In Progress

Resources from the private repository have been recently imported. The build is not yet functional and instructions will be provided once integration is complete.

---

<p align="center">
	<img width="400" alt="Blip icon" src="misc/icon.png">
</p>

<!-- ![CI](https://github.com/bliporg/blip/actions/workflows/ci.yml/badge.svg) -->
[![Join the chat at https://discord.gg/blipgame](https://img.shields.io/discord/355905150528913409?color=%237289DA&label=blip&logo=discord&logoColor=white)](https://discord.gg/blipgame)

## What is Blip?

Blip is a Roblox-like platform tailored for generative AI, empowering a wide audience of creators to make games. It even allows games to be created on mobile:

<p align="center">
<img width="90%" alt="Blip demo" src="misc/img/blip-demo.gif">
</p>

While you can build game logic through natural language, developers still have full access to write and modify code directly.

## Lightweight, All-In-One & Cross-Platform

All features are bundled into a single cross-platform application - there's no need for a separate "studio" app for creators.

**Supported platforms:** iOS/iPadOS, Android, Windows, macOS, Web Browsers & Discord

## Fully scriptable

- Experiences in Blip are scripted in [Luau](https://luau.org), a fast, small, safe, gradually typed embeddable scripting language derived from [Lua](https://www.lua.org)
- Developers can script both client and server-side logic, with free scalable server infrastructure for real-time multiplayer
- Core APIs are documented at [docs.blip.game/reference](https://docs.blip.game/reference)
- Extend functionality with open-source [modules](https://docs.blip.game/modules) hosted on GitHub. Here's an example:

	```luau
	Modules = {
		fire = "github.com/aduermael/modzh/fire"
	}

	Client.OnStart = function()
		Player:SetParent(World)
		Camera:SetModeThirdPerson()
	
		local f = fire:create()
		f:SetParent(Player)
		-- now Player is on fire
	end
	```
	<p align="center">
		<img width=50% alt="" src="misc/img/fire.gif">
	</p>
	
- The [API documentation](https://docs.blip.game) is generated from the [lua](https://github.com/bliporg/blip/tree/main/lua) folder in this repository
	
## Blip Engine

Blip is a C/C++ in-house game engine that uses [BGFX](https://github.com/bkaradzic/bgfx) for cross-platform rendering.
It compiles natively for each platform and uses [WebAssembly](https://webassembly.org) to support web browsers and other web app platforms like [Discord](https://discord.com).

## Development

Most communication among contributors, players, and creators takes place on the [official Discord server](https://discord.gg/blipgame).


