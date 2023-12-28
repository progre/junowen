# Ju.N.Owen

[日本語のドキュメントはこちら](./README.ja.md)

This is an unofficial online battle tool for Touhou Juuouen ~ Unfinished Dream of All Living Ghost (Touhou 19 UDoALG).

It is an unofficial tool. **Use at your own risk**.

This tool realizes online battles with its own mechanism, which is different from the matching and synchronization mechanism of official online battles.
It works in the same way as adonis and th075caster.

## Features

- Less likely to be out of alignment than official online battles
- Delay can be changed during the game
- Can be connected without a server
- Can spectate the game

## How to install

1. Extract the zip file and make sure you have d3d9.dll and th19_junowen.dll in the modules folder.
2. Open the Touhou 19 installation folder.
3. Move d3d9.dll and the modules folder into Touhou 19 folder.
4. Start Touhou 19.
5. If all goes well, "Ju.N.Owen" will be added as an item on the title screen of Touhou 19.
6. If you get the error "VCRUNTIME140.dll is missing..." and it won't start,
   install the x86 version of
   Visual C++ Redistributable for Visual Studio 2015 (vc_redist.x86.exe).
   <https://www.microsoft.com/en-us/download/details.aspx?id=48145>

## How to use

Three connection methods are currently supported.

### Shared Room

This method connects to users whose room name matches the set room name.  
While waiting for a connection, other functions can be used.

The room name should be set in "Online VS Mode".

Pressing the shot button on the waiting for connection screen interrupts, and pressing the cancel button allows you to use other functions.

### Reserved Room

This method connects to users whose room name matches the set room name.  
You can have other players spectate your matches.

### Pure P2P

This method does not use a connection server, but exchanges connection information with opponents via chat or other means.

#### Using Pure P2P competition

1. Select "Ju.N.Owen" -> "Pure P2P”.
2. Select "Connect as a Host" if you want to wait for a connection as a host,
   Select "Connect as a Guset" to connect as a guest.
    - Host
        1. A long string `<offer>********</offer>` will be displayed and automatically copied to the clipboard,
           Send this string to your opponent using Discord or other means.
           Select "Copy your code" to copy it to the clipboard again.
        2. Receive the string `<answer>********</answer>` from your opponent,
           Copy it to the clipboard.
        3. Select "Paste guest's code".
        4. If all goes well, you will be redirected to the difficulty selection and the game will begin.
    - Guest
        1. Receive the string `<offer>********</offer>` from your opponent and copy it to the clipboard.
        2. Press the shot button to enter the clipboard contents.
        3. Take the long string `<answer>********</answer>` and automatically copy it to the clipboard,
           Send this string to your opponent via Discord or other means.
           Press the shot button to copy the string to the clipboard again.
        4. If all goes well, you will be redirected to the difficulty selection screen and the game will begin.

#### Using Pure P2P spectate

- Spectator
    1. Select “Ju.N.Owen" -> "Pure P2P” -> "Connect as a Spectator"
    2. A long string `<s-offer>********</s-offer>` will be displayed and automatically copied to the clipboard,
       Send this string to one of the players via Discord or other means.
       Select "Copy your code" to copy it to the clipboard again.
    3. Receive the string `<s-answer>********</s-answer>` from the player,
       copy it to the clipboard.
    4. Select "Paste guest's code"
    5. If all goes well, the game will start.
    6. Press the pause button to stop the spectating.
- Player
    1. Connect to the opponent via Ju.N.Owen's match function and wait for the difficulty level selection.
    2. receive the string `<s-offer>********</s-offer>` from the spectator and copy it to the clipboard
    3. Press the F1 key to enter the clipboard contents.
    4. Take the long string `<answer>********</answer>` and automatically copy it to the clipboard,
       Send this string to your opponent via Discord or other means.
    5. If all goes well, you can let them spectate the game.

### After connection

- During the connection, the names of both parties are displayed at the top of the screen. When disconnected, the display will disappear.
- The host can change the delay value with the number keys 0-9 during the game.

## Supplement

- No ports need to be open.
- Even if a port is open, that port cannot be specified.

## Current constraints

- "Online VS Mode" must be released for the game to work properly.
- Ju.N.Owen menu cannot be operated with the enter key
- Spectators can only be added immediately after a player connects.
- The game may be freeze if communication is delayed or something not good happens.

## Author and distributor

[Progre](https://bsky.app/profile/progre.me)

<https://github.com/progre/junowen>
