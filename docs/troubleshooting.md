- [Wrong map detection](#wrong-map-detection)
- [Actions contention](#actions-contention-)
- [Default Ratio game resolution](#default-ratio-game-resolution)
- [Preventing double jump(s)](#preventing-double-jumps)
- [Up jump key](#up-jump-key)
- [Missing installation](#missing-installation)
- [Unstucking state](#unstucking-state)

## Wrong map detection
Wrong map detection can happen when:
- Moving quickly between different maps
- Other UIs overlapping
- The map is not expanded fully

Rule of thumb is:  Map detection will always try to crop the white border and make it is as tight as possbile. So before
creating a new map, double-check to see if the map being displayed has white border cropped and looks similar to the
one in the game.

Fix methods:
- Below the map are three buttons, two of which can be used to help troubleshooting:
    - `Re-detect map`: Use this button to re-detect the map
    - `Delete map`: Use this to **permanently delete** the map
- Move the map UI around
- When moving around different maps, it may detect previous map due to delay. Just use `Re-detect map` 
button for this case.

## Actions contention (?)
Action with `EveryMillis` can lead to contention if you do not space them out properly. For example, if there are two `EveryMillis` actions executed every 2 seconds, wait 1 second afterwards and one normal action, it is likely the normal action will never
get the chance to run to completion.

That said, it is quite rare.

## Default Ratio game resolution
Currently, the bot does not support `Default Ratio` game resolution because most detection resources are
in `Ideal Ratio` (1920x1080 with `Ideal Ratio` or 1376x768 below). `Default Ratio` currently only takes effect
when play in `1920x1080` or above, making the UI blurry.

## Preventing double jump(s)
**This is subject to change** but if you want to the bot to only walk between points then the two
points `x` distance should be less than `25`.

## Up jump key
In general, this key is optional and meant for classes that have a separate skill to up jump. Below is details
on how to set up this key.

- If you are a mage class:
  - You need to set the `Teleport key`
  - If you have up jump, which most mage classes now have (e.g. holding up arrow + jump key), you don't need to set this key
  - If you have a dedicated up jump key (e.g. similar to Hero up jump), you should set this key
  - The bot will try to use the following combinations where appropriate:
    - Teleport only
    - Jump and then teleport
    - Up jump and then teleport
- If you are Demon Slayer, set this key to up arrow
- If you are any other class with up jump skill such as Explorer Warriors, Blaster,... set this key to that skill:
  - If your up jump is too short and can be used mid-air (e.g. Night Lord), enable `Jump then up jump if possible`
- If your up jump is through flying (e.g. Illium, Hoyoung), enable `Up jump is flight`

## Missing installation
If you use the bot on a newly installed Windows, make sure [Visual C++ Redistributable 2015-2022](https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist#visual-studio-2015-2017-2019-and-2022) and [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2?form=MA13LH) are installed.

## Unstucking state
Unstucking state is a state that helps the bot from being stuck by UI dialog, undetectable player position, rope, etc..
However, this state can also transition due to wrong setup. This state can be caused by the following main reason:
- The bot successfully detects the minimap but failed to detect the player and thinks the player might be inside the edges
- The bot tries to perform action that requires movement but the player did not move after a while
- When using remote control app:
  - Usually the Num Lock key can cause the bot to send `4826` instead of arrow keys in the `Default Input Method` and bot will keep moving in one direction caused by pressing `Jump key` without arrow keys
  - Using the bot with through remote control requires precise game window size on the host (the PC that runs the bot), check the remote control documentation for more details


