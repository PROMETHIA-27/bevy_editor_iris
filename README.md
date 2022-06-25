# bevy_editor_iris

An experiment towards a networked editor for Bevy Engine. 
Currently not functional as an editor.

#### Planned features:
- Editor is as simple to install as a cargo dependency
- Editor runs separately from the game, and can stay alive while the game is restarted
- Editor connects to the game via QUIC protocol
- Launch the editor on one machine, the game on another, and edit as normal
- Ideally, cross-platform support so that a game running on a console or mobile device can be edited from a PC

If you have any expertise in networking or editor creation, feel free to lend a hand! Especially let me know if there's something obviously wrong; I do not have a lot of networking experience yet, nor knowledge of networking insecurities.

Dual-licensed as either MIT or Apache 2.0
