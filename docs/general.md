# This content is AI-generated with provision

- [Download](#download)
- [Concepts](#concepts)
  - [Map](#map)
  - [Movement](#movement)
  - [Characters](#characters)
  - [Action](#action)
      - [Normal and Priority](#normal-and-priority)
      - [Configuration](#configuration)
  - [Linked Key & Linked Action](#linked-key--linked-action)
  - [Rotation Modes](#rotation-modes)
    - [Auto-mobbing](#auto-mobbing)
    - [Ping Pong](#ping-pong)
  - [Platforms Pathing](#platforms-pathing)
  - [Navigation](#navigation)
  - [Run/Stop Cycle](#runstop-cycle)
  - [Capture Modes](#capture-modes)
  - [Familiars Swapping](#familiars-swapping)
  - [Panic Mode](#panic-mode)
  - [Elite Boss Spawns Behavior](#elite-boss-spawns-behavior)
  - [Control and Notifications](#control-and-notifications)
- [Video Guides](#video-guides)
- [Showcase](#showcase)
  - [Rotation](#rotation)
  - [Auto Mobbing & Platforms Pathing](#auto-mobbing--platforms-pathing)
  - [Rune Solving](#rune-solving)

## Download

1. Go to the [GitHub Release Page](https://github.com/sasanquaa/komari/releases)
2. Download `app-release-gpu.zip` or `app-release-cpu.zip`
3. Extract the archive
4. Run the `.exe` file

## Concepts

### Map

- The map is detected automatically but must be created manually by providing a name.
- The created map is saved and can be selected later.
- Any action presets created in the detected map are saved to that map only.

> **Note:**  
> The arcs shown are for visualization purposes only. They do **not** represent the actual movement path. However, they indicate the sequence of actions, as shown by their numbering.

![Map](https://github.com/sasanquaa/komari/blob/master/.github/images/map.png?raw=true)

### Movement

Default bot movement (without platform pathing) follows these steps:

1. Moves horizontally to match the destination’s `x` coordinate.
2. Then performs a fall, up-jump, or grapple to match the destination’s `y` coordinate.

If the bot is close enough to the destination (within `25` units, subject to change), it will **walk** instead of performing a double jump.

### Characters

- The `Characters` tab is used to change key bindings, set up buffs, and more.
- Character profiles can be created separately for each character.
- Characters are saved globally and are not tied to a specific map.

#### Character Sections

1. `Key Bindings` – For general in-game key mappings.
2. `Feed pet` – Configures feed pet.
3. `Use potion` – Configures potion usage.
4. `Use booster` – Configures VIP/HEXA booster usage.
5. `Movement` – Movement settings.
6. `Buffs` – For automatic buff setup.
7. `Fixed Actions` – Shared across all maps (useful for buffs or one-time skills).
8. `Others` – Miscellaneous character settings.

##### Potion Mode

There are two modes available for configuring potion usage:

- `EveryMillis` – Uses a potion every `x` milliseconds.  
- `Percentage` – Uses a potion when HP drops below a certain percentage.

##### Movement

- `Disable teleport on fall` – Disables teleport after falling (useful for mage classes).
- `Disable double jumping` – Disables the `DoubleJumping` state (e.g., makes the bot only walk).  
  - Works only if the action does not have `Use with = DoubleJump`.
- `Disable walking` – Disables the `Adjusting` state (forces horizontal movement by double jumps only).  
  - Works only if the action does not have `Adjust` ticked.

From **v0.12**, the `Rope Lift` skill can now be disabled. If not provided, the bot will attempt to up-jump instead.

> **Note:**  
> For supported buffs, the bot relies on detecting buffs in the top-right corner of the screen.

![Buffs](https://github.com/sasanquaa/komari/blob/master/.github/images/buffs.png?raw=true)

From **v0.21**, if multiple conflicting buffs (e.g., x2/x3 EXP coupons, small/large WAP) are enabled,  
the bot will prevent buffing one if it detects the other is active. Currently, it does **not** prioritize which buff to use first.

### Action

Can be configured under `Actions` tab. Action is the bot main mechanism for using skills, solving rune, buffing, etc.

There are two types of actions:

- `Move` – Moves to a specific location on the map.  
- `Key` – Uses a key (optionally with a position).

#### Normal and Priority

Actions are categorized as either **normal** or **priority**:

- **Priority action** overrides normal action temporarily.  
- Normal action resumes once the priority action is complete.

Current priority actions include:
- `Erda Shower off cooldown`
- `Every milliseconds`

> **For `Erda Shower off cooldown` to work:**  
> - The Erda Shower skill must be assigned to a quick slot.  
> - Action customization must be toggled on and visible.  
> - The skill must actually cast when triggered, or the bot will rerun the actions chain.

![Erda Shower](https://github.com/sasanquaa/komari/blob/master/.github/images/erda.png?raw=true)

#### Configuration

For `Move` action:
- `Adjust` – Ensures the actual position matches the target closely.  
  - When enabled, it overrides the `Disable walking` option and allows walking.  
- `X` – Horizontal coordinate to move to.  
- `X random range` – Adds randomization: `[x - range, x + range]`.  
- `Y` – Vertical coordinate to move to.  
- `Wait after move` – Delay (in milliseconds) after moving (e.g., for looting).  
- `Linked action` – See [Linked Key & Linked Action](#linked-key--linked-action).  
  - Can only be used if it is not the first action or the list is non-empty.

For `Key` action:
- `Positioned` – Determines if the key action is position-dependent.  
- `X / X random range / Y / Adjust / Linked action` – Same as Move Action.  
- `Key` – The key to press.  
- `Use count` – Number of times to use the key.  
- `Has link key` – Enables link key (useful for combo classes).  
- `Queue to front` – For priority actions only.  
  - Allows this action to override non-queue-to-front priority actions.  
  - The overridden action is delayed, not lost.  
  - Useful for `press attack after X ms even while moving`.  
  - Cannot override linked actions.  
- `Use direction` – Sets the direction for the action.  
- `Use with` - Uses the key with specific player's state.  
  - `Any` – Performs as appropriate.  
  - `Stationary` – Only when standing (for buffs).  
  - `DoubleJump` – With double jump.  
- `Wait before / Wait after` – Delay (ms) before/after using the key (applies to each repeat).  
- `Wait random range` – Adds randomness to the wait time: `[delay - range, delay + range]`.

Actions can be reordered using the up/down icons.

![Actions](https://github.com/sasanquaa/komari/blob/master/.github/images/actions.png?raw=true)

### Linked Key & Linked Action

Useful for combo-oriented classes such as Blaster, Cadena, Ark, Mercedes, etc. Animation cancel timings
depend on the class, which can be configured in `Characters` → `Others` → `Link key timing class`.

#### Link Key Types

- `Before` – Uses the link key before the main key (e.g., Cadena’s Chain Arts: Thrash).  
- `AtTheSame` – Uses both keys simultaneously (e.g., Blaster skating).  
- `After` – Uses the link key after the main key (e.g., Blaster Weaving/Bobbing).  
- `Along` – Uses the link key along with the main key while the link key is being held down (e.g., in-game Combo Key).

> **Notes:**  
> - Even for `AtTheSame`, the link key is sent **first**.  
> - For Blaster, if `After` is set and the link key is not `Jump`, an extra `Jump` will be sent to cancel Bobbing/Weaving.  
> - Linked key can also be simulated via linked actions.
> - `Along` type timing is fixed and unaffected by class.

#### Linked Actions

You can chain actions by enabling `Linked action` on subsequent ones.  
The first action starts the chain:

![Linked Actions](https://github.com/sasanquaa/komari/blob/master/.github/images/linked_actions.png?raw=true)

Linked actions appear visually connected with vertical bars. Once a chain begins, it cannot be overridden by any other actions.

### Rotation Modes

Rotation mode defines how **normal actions** are executed (priority actions are unaffected).  
You can select the mode in `Actions` → `Rotation`.

#### Available Modes

- `StartToEnd` – Runs actions from start to end and repeats.  
- `StartToEndThenReverse` – Runs from start to end, then reverses order.  
- `AutoMobbing` – Ignores normal actions; automatically detects and attacks mobs.  
- `PingPong` – Ignores normal actions; moves and attacks between bounds.

Priority actions (`Every milliseconds` and `Erda Shower off cooldown`) still follow their own logic.

#### Auto-Mobbing

Auto-mobbing targets random mobs detected on screen. To use it, sets the rotation mode to `AutoMobbing` and updates
the mobbing bound.

For platform-based movement, see [Platforms Pathing](#platforms-pathing). If platforms are added:
- Uses them to better estimate mob positions.
- Identifies platform "gaps" to avoid invalid mob locations.
- Adding platforms is encouraged but not required. Without them, the bot relies more on randomness.

From **v0.18.0**, Auto-mobbing now follows a fixed **clockwise** path, improving mob count on large maps.

How it works:
1. The bound is divided into four quadrants.  
2. The player moves between quadrants clockwise.  
3. Only mobs in the current quadrant are targeted.  
4. If no mobs remain, it moves to the next quadrant.

![Auto-mobbing](https://github.com/sasanquaa/komari/blob/master/.github/images/auto_mobbing.png?raw=true)

From **v0.21.0**, two new options are added:
- `Auto mobbing uses key when pathing` – Uses mobbing key while moving between quadrants and mobs are detected ahead.  
- `Detect mobs when pathing every` – Sets mobs detection interval when moving between quadrants.

#### Ping Pong

Introduced in **v0.12**.

Ping pong makes the player double jumps and attacks inside the bound, reversing direction upon reaching the bound edge. To use it, sets the rotation mode to `PingPong` and updates the mobbing bound.

This mode will try to force the player to always be within the bound. If player is already inside the bounds:
- May grapple or up-jump when below bound's `y` midpoint.  
- May fall when above bound's `y` midpoint.  
- Within `9` units of bound's `y` midpoint, there will be no random movement.

Simpler than Auto-mobbing; suitable for classes that primarily jump and spam attacks (e.g., Night Walker).


### Platforms Pathing

Supported for Auto-mobbing and Rune solving. This feature helps pathing across platforms, with
or without the `Rope Lift` skill. To use, add platforms for the selected map starting from ground level.
Use hotkeys to add them quickly.

> **Note**:
> Adding platforms improves Auto-mobbing movement.

### Navigation

Introduced in **v0.19**.

Enables the bot to navigate automatically between maps using portals.

#### Core Concepts

- `Paths group` – A collection of related paths (e.g., Hotel Arcus).  
- `Path` – A minimap snapshot containing its name, minimap images and coordinates (e.g., portals).  
- `Point` – A transition marker to another path.

#### Setup Steps

1. Opens the `Navigation` tab.  
2. Notes down the desired route (e.g., Esfera Base Camp → Esfera Mirror-touched Sea 3).  
3. Creates a `Paths group` (e.g., Esfera).  
4. Goes to each map and ensures the minimap is detected.  
5. Clicks `Add path` – captures the minimap and name images for matching.  
6. Clicks `Add point` – records portal coordinates and links to next path.  
7. Repeats from 3. until all paths are added.  
8. Attaches a created path to the current map under `Navigation` → `Selected map` → `Attached paths group` and `Attached path`.

When started, the bot will navigate to the attached path before rotating actions.  
Useful for:
- Run/stop cycles (e.g., returns to town on stop cycle and navigates back).  
- Navigates back to original map if the bot changes map accidentally.

#### Limitations

- No interaction-based navigation yet.  
- Cannot handle portals leading to `Unstucking` state positions.

![Navigation](https://github.com/sasanquaa/komari/blob/master/.github/images/navigation.png?raw=true)

From **v0.21**, added `Use grayscale for map` option for better minimap matching if color-based detection fails.

### Run/Stop Cycle

Introduced in **v0.19**.

Found under `Settings` → `Run/stop cycle`:

- `None` – Runs or stops indefinately (default behavior).  
- `Once` – Runs for a specified duration, then stops and returns to town.  
- `Repeat` – Alternates between running and resting in town.

**For this to work:**
- Key binding for `To town` is set.  
- If `Repeat` mode is used, navigation paths must be setup.

The `Suspend` button allows pausing temporarily without resetting timer.

### Capture Modes

Found in `Settings` → `Capture` → `Mode`.

Defines how the bot captures the game image. Three modes available:

1. `BitBlt`.  
2. `Windows 10 (1903 and up)` - Default mode.  
3. `BitBltArea` – Captures a fixed region.  
   - Useful for VMs, capture cards, or Sunshine/Moonlight setups.  
   - The capture area must always include the game window.  
   - The capture area can stay behind the game but cannot be minimized.  
   - **The game must always be contained inside the capture area even when the game resizes.**  
   - **Key inputs are sent to the focused window above the capture area**.

You can also directly select a capture window via `Handle`.

### Familiars Swapping

Introduced in **v0.13**.

Located under `Settings` → `Familiars`. Automatically checks equipped familiar levels and swaps them when maxed:

- `Swap check every` – Interval in milliseconds between checks.  
- `Swappable slots`.  
  - `All` – All slots.  
  - `Last` – Only last slot.  
  - `SecondAndLast` – Second and last slots only.  
- `Can swap rare familiars` – Allows rare ones to be swapped.  
- `Can swap epic familiars` – Allows epic ones to be swapped.

Supports scrolling through familiar cards.  
For best results, keep available cards visible without scrolling.

> **Notes:**  
> - After swapping, familiar buff will be turned off. To enable familiar buff again, enables it in the `Buffs` tab.  
> - All familiar slots must be unlocked, and the familiar menu key binding must be set.

### Panic Mode

Introduced in **v0.14**.

Located under `Settings` → `Panic mode`.

If another player (friend, guildmate, or stranger) appears on the same map for 15 seconds,  
the bot enters `Panicking` state and cycles through channels until a map without any other player is found. 
Requires `Change channel` key binding.

From **v0.18**, if `Stop actions on fail or map change` is also enabled, the bot stops and goes to town upon failure. 
Requires `To town` key binding.

### Elite Boss Spawns Behavior

Introduced in **v0.17**, previously known as `Change channel on Elite Boss`.

Found under `Characters` → `Elite Boss spawns behavior`.

Available behaviors:
- `None` – No action.  
- `CycleChannel` – Changes channel when an Elite Boss appears.  
- `UseKey` – Triggers a key (useful for origin skills).

### Control and Notifications

Uses Discord webhook or bot token for notifications and remote control.

#### Notifications

Provide a **webhook URL** (for notifications only) or a **bot token** (for full control).

Available notification types:
- Rune Spawns  
- Elite Boss Spawns  
- Player Dies  
- Guildie Appears  
- Stranger Appears  
- Friend Appears  
- Detection Fails / Map Changes

If `Discord ping user ID` is set, the bot pings that user in the notification message.

#### Discord Commands

Introduced in **v0.20**.

- `/status` – Shows current status, runtime, and image.  
- `/start` – Starts the bot.  
- `/stop` – Stops the bot (optionally goes to town).  
- `/suspend` – Pauses temporarily (or fully if no cycle is active).  
- `/start-stream` – Streams status periodically (up to 15 min).  
- `/stop-stream` – Stops streaming.  
- `/chat` – Sends in-game chat (ASCII only).  
- `/action` – Performs a specified action (with kind and count).

> The Discord bot is experimental and may change.

## Video Guides

From v0.16 — the first two videos are outdated but still useful for basics.

1. [Basic Operations](https://youtu.be/8X2CKS7bnHY?si=3yPmVPaMsFEyDD8c)  
2. [Auto-Mobbing and Platforms Pathing](https://youtu.be/8r2duEz6278?si=HTHb8WXh6L7ulCoE)  
3. Rotation Modes, Linked Key & Linked Actions (TODO)  
   - [Clockwise Rotation Example](https://youtu.be/-glx3b0jGEY?si=nuEDmIQTuiz3LtIq)

## Showcase (Examples from v0.1)

### Rotation
https://github.com/user-attachments/assets/3c66dcb9-7196-4245-a7ea-4253f214bba6

(This Blaster rotation was before Link Key & Link Action were added)

https://github.com/user-attachments/assets/463b9844-0950-4371-9644-14fad5e1fab9

### Auto-Mobbing & Platforms Pathing
https://github.com/user-attachments/assets/3f087f83-f956-4ee1-84b0-1a31286413ef

### Rune Solving
https://github.com/user-attachments/assets/e9ebfc60-42bc-49ef-a367-3c20a1cd00e0
