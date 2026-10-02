<img src='reasonate-github-header.jpg' width='100%'>

# ***Resonate***

> [!IMPORTANT]
> This project was originally developed and maintained by Huge Menace. **As of Jan 2026, it is officially maintained by Widgit Gaming**. Future updates, issues, and discussions are hosted at 👉 [gitlab.com/widgitgaming/godot/resonate](https://gitlab.com/widgitgaming/godot/resonate) — Thank you to the community for your support!

An all-in-one sound and music management addon for the Godot game engine (see compatibility).

## Features
- Pooled audio stream players.
- Automatic 2D and 3D space detection.
- Polyphonic playback.
- Stemmed music tracks.
- Music crossfading.

## TL;DR

Resonate has two core systems: the **SoundManager** and the **MusicManager**.

The **SoundManager** automatically pools and orchestrates **AudioStreamPlayers** for you and gives you control over the players when needed. 

The **MusicManager** composes music tracks built from ***stems*** and supports the (cross)fading of tracks or stems out of the box.

## Docs

- [Getting started](docs/getting-started.md)
- [SoundManager documentation](docs/sound-manager.md)
- [MusicManager documentation](docs/music-manager.md)

## Examples

This repo is a Godot project you can clone and run.

Inside the `examples/` folder are scenes demonstrating all the Sound and Music Manager's features.

## Getting the addon

You have a few different options:

- You can download Resonate from the [Godot Asset Library](https://godotengine.org/asset-library/asset/2546) (or from within the editor). 
- You can grab the latest version from the [Github releases page](https://github.com/hugemenace/resonate/releases).
- You can also clone or download this repo, and extract the `addons/resonate` directory into the root folder of your Godot project.
- You can grab the addon from [Gumroad](https://hugemenace.gumroad.com/l/resonate-godot-addon) (if you'd like to financially support this project).

## License

This project is provided ***free for personal and commercial use*** under the [MIT license](LICENSE) ❤
