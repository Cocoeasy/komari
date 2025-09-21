import pywinauto
import pyautogui
import grpc
import ctypes

user32 = ctypes.windll.user32

from concurrent import futures
from pywinauto import WindowSpecification, keyboard
from pywinauto.application import Application

# The two imports below is generated from:
# python -m grpc_tools.protoc --python_out=. --pyi_out=. --grpc_python_out=. -I../../backend/proto ../..
# /backend/proto/input.proto
from input_pb2 import (
    Key,
    KeyRequest,
    KeyResponse,
    KeyDownRequest,
    KeyDownResponse,
    KeyUpRequest,
    KeyUpResponse,
    KeyInitRequest,
    KeyInitResponse,
    MouseRequest,
    MouseResponse,
    MouseAction,
    Coordinate,
    KeyState,
    KeyStateRequest,
    KeyStateResponse,
)
from input_pb2_grpc import KeyInputServicer, add_KeyInputServicer_to_server


class KeyInput(KeyInputServicer):
    def __init__(
        self,
        window: WindowSpecification,
        keys_map: dict[Key, str],
        vk_keys_map: dict[Key, int],
    ) -> None:
        super().__init__()
        self.window = window
        self.keys_map = keys_map

    # This is the init function that is called each time the bot connects to your service.
    def Init(self, request: KeyInitRequest, context):
        # This is a seed generated automatically by the bot for the first time the bot is run.
        # The seed is saved in the database and reused again later.
        # If you do not wish to use the bot provided delay for key down press, you can use this
        # seed for generating delay timing. The seed is a 32 bytes array.
        # self.seed = request.seed

        # There are two types of mouse coordinate depending on your setup:
        # - Relative: The MouseRequest coordinates (x, y, width, height) is relative to the
        #   current window the bot is capturing. For example, if you play your game in 1366x768,
        #   then (width, height) = (1366, 768) and (x, y) is offset from top-left corner of that
        #   window with (0, 0) being top-left and (width, height) is bottom-right.
        #
        # - Screen: The MouseRequest coordinates (x, y, width, height) is relative to the
        #   current monitor screen of the app the bot is capturing (which monitor the app is in).
        #   With (0, 0) being top-left of that monitor screen and (width, height) is bottom-right.
        #   For example, your game might be (1366, 768) but it is running in the monitor of size
        #   (1920, 1080) so (width, height) = (1920, 1080).
        #
        # You should return the one appropriate for your setup in this Init() function.
        # return KeyInitResponse(mouse_coordinate=Coordinate.Screen)
        return KeyInitResponse(mouse_coordinate=Coordinate.Relative)

    def KeyState(self, request: KeyStateRequest, context):
        is_down = (user32.GetAsyncKeyState(self.vk_keys_map[request.key]) & 0x8000) != 0
        if is_down:
            return KeyStateResponse(KeyState.Pressed)
        else:
            return KeyStateResponse(KeyState.Released)

    def SendMouse(self, request: MouseRequest, context):
        # Regardless of the type of Coordinate return in Init(), the coordinates are always based on
        # the PC the bot is running in. And there are two cases you should consider:
        #
        # - If you run this server on a separate PC than the bot PC and use remote control, this
        #   coordinate is NOT local to the server PC
        #
        # - If you run this server on the same PC as the bot, this coordinate is local to
        #   the server PC
        #
        # The coordinates x, y represent the location the bot wants the input server to click
        # relative to the PC the bot is in. Therefore, it must be transformed first to match your
        # current setup and also to the x, y values your input method can use.
        #
        # For example, KMBox requires x, y values to be relative while SendInput requires
        # the x, y values to be absolute in the range [0, 65535].
        width = request.width
        height = request.height
        x = request.x
        y = request.y
        action = request.action

        # pywinauto mouse requires absolute screen coordinate.
        #
        # Case 1: pyautogui input server is in the same PC as bot. Just use Coordinate.Screen is
        # enough.

        # Case 2: pyautogui input server is in a different PC than the bot. This case can be
        # problematic depending on your setup. For instance, if you use GF Now, when running
        # the game, there are no "status bars" or other non-game UI areas. Your game will always show
        # without any kind of border/bars that might inset the actual game. But if you run your
        # game in something like a VM or Sunshine/Moonlight, these apps can have these
        # bars and the bot always capture the full app and not just the game being shown. So
        # you need to subtract the coordinates by some amount until it feels "correct".
        #
        # You need to use Coordinate.Relative for this case.

        # These are for cropping the non-game UI portion of the app the game is running in.
        # For Moonlight/Sunshine, you can leave this as-is. This method can be unreliable due
        # this reason. You can also use PowerToys Screen Ruler to measure this non-game UI area.
        crop_left_px = 0  # Change this until it feels correct
        crop_top_px = 30  # Change this until it feels correct

        game_width = 1366  # Assuming your game is 1366x768 full screen
        game_height = 768  # Assuming your game is 1366x768 full screen
        x = int(((x - crop_left_px) / (width - crop_left_px)) * game_width)
        y = int(((y - crop_top_px) / (height - crop_top_px)) * game_height)

        # Common logics, not very human but just an example
        if action == MouseAction.Move:
            pyautogui.moveTo(x, y)
        elif action == MouseAction.Click:
            pyautogui.click(x, y)
        elif action == MouseAction.ScrollDown:
            pyautogui.moveTo(x, y)
            pyautogui.scroll(-200)

        return MouseResponse()

    def Send(self, request: KeyRequest, context):
        if self.window.has_keyboard_focus():
            # This `key` is an enum representing the key the bot want your customized input to send.
            # You should map this to the key supported by your customized input method.
            key = self.keys_map[request.key]
            # This is key down sleep milliseconds. It is generated automatically by the bot using the
            # above seed. You should use this delay and `time.sleep(delay)` on key down.
            key_down = request.down_ms / 1000.0

            keyboard.send_keys("{" + key + " down}", pause=key_down, vk_packet=False)
            keyboard.send_keys("{" + key + " up}", pause=0, vk_packet=False)

        return KeyResponse()

    def SendUp(self, request: KeyUpRequest, context):
        if self.window.has_keyboard_focus():
            keyboard.send_keys(
                "{" + self.keys_map[request.key] + " up}", pause=0, vk_packet=False
            )
        return KeyUpResponse()

    def SendDown(self, request: KeyDownRequest, context):
        if self.window.has_keyboard_focus():
            keyboard.send_keys(
                "{" + self.keys_map[request.key] + " down}", pause=0, vk_packet=False
            )
        return KeyDownResponse()


if __name__ == "__main__":
    window_args = {"class_name": "MapleStoryClass"}
    window = (
        Application()
        .connect(handle=pywinauto.findwindows.find_window(**window_args))
        .window()
    )
    # Generated with ChatGPT, might not be accurate
    keys_map = {
        # Letters
        Key.A: "a",
        Key.B: "b",
        Key.C: "c",
        Key.D: "d",
        Key.E: "e",
        Key.F: "f",
        Key.G: "g",
        Key.H: "h",
        Key.I: "i",
        Key.J: "j",
        Key.K: "k",
        Key.L: "l",
        Key.M: "m",
        Key.N: "n",
        Key.O: "o",
        Key.P: "p",
        Key.Q: "q",
        Key.R: "r",
        Key.S: "s",
        Key.T: "t",
        Key.U: "u",
        Key.V: "v",
        Key.W: "w",
        Key.X: "x",
        Key.Y: "y",
        Key.Z: "z",
        # Digits
        Key.Zero: "0",
        Key.One: "1",
        Key.Two: "2",
        Key.Three: "3",
        Key.Four: "4",
        Key.Five: "5",
        Key.Six: "6",
        Key.Seven: "7",
        Key.Eight: "8",
        Key.Nine: "9",
        # Function Keys
        Key.F1: "F1",
        Key.F2: "F2",
        Key.F3: "F3",
        Key.F4: "F4",
        Key.F5: "F5",
        Key.F6: "F6",
        Key.F7: "F7",
        Key.F8: "F8",
        Key.F9: "F9",
        Key.F10: "F10",
        Key.F11: "F11",
        Key.F12: "F12",
        # Navigation and Controls
        Key.Up: "UP",
        Key.Down: "DOWN",
        Key.Left: "LEFT",
        Key.Right: "RIGHT",
        Key.Home: "HOME",
        Key.End: "END",
        Key.PageUp: "PGUP",
        Key.PageDown: "PGDN",
        Key.Insert: "INSERT",
        Key.Delete: "DEL",
        Key.Esc: "ESC",
        Key.Enter: "ENTER",
        Key.Space: "SPACE",
        # Modifier Keys
        # control (can also be '{VK_CONTROL}' if needed)
        Key.Ctrl: "VK_CONTROL",
        Key.Shift: "VK_SHIFT",  # shift (can also be '{VK_SHIFT}')
        Key.Alt: "VK_MENU",  # alt (can also be '{VK_MENU}')
        # Punctuation & Special Characters
        Key.Tilde: "`",
        Key.Quote: "'",
        Key.Semicolon: ";",
        Key.Comma: ",",
        Key.Period: ".",
        Key.Slash: "/",
    }

    vk_keys_map = {
        # Letters (A–Z)
        Key.A: 0x41,
        Key.B: 0x42,
        Key.C: 0x43,
        Key.D: 0x44,
        Key.E: 0x45,
        Key.F: 0x46,
        Key.G: 0x47,
        Key.H: 0x48,
        Key.I: 0x49,
        Key.J: 0x4A,
        Key.K: 0x4B,
        Key.L: 0x4C,
        Key.M: 0x4D,
        Key.N: 0x4E,
        Key.O: 0x4F,
        Key.P: 0x50,
        Key.Q: 0x51,
        Key.R: 0x52,
        Key.S: 0x53,
        Key.T: 0x54,
        Key.U: 0x55,
        Key.V: 0x56,
        Key.W: 0x57,
        Key.X: 0x58,
        Key.Y: 0x59,
        Key.Z: 0x5A,
        # Digits (0–9)
        Key.Zero: 0x30,
        Key.One: 0x31,
        Key.Two: 0x32,
        Key.Three: 0x33,
        Key.Four: 0x34,
        Key.Five: 0x35,
        Key.Six: 0x36,
        Key.Seven: 0x37,
        Key.Eight: 0x38,
        Key.Nine: 0x39,
        # Function Keys
        Key.F1: 0x70,
        Key.F2: 0x71,
        Key.F3: 0x72,
        Key.F4: 0x73,
        Key.F5: 0x74,
        Key.F6: 0x75,
        Key.F7: 0x76,
        Key.F8: 0x77,
        Key.F9: 0x78,
        Key.F10: 0x79,
        Key.F11: 0x7A,
        Key.F12: 0x7B,
        # Navigation & Controls
        Key.Up: 0x26,
        Key.Down: 0x28,
        Key.Left: 0x25,
        Key.Right: 0x27,
        Key.Home: 0x24,
        Key.End: 0x23,
        Key.PageUp: 0x21,
        Key.PageDown: 0x22,
        Key.Insert: 0x2D,
        Key.Delete: 0x2E,
        Key.Esc: 0x1B,
        Key.Enter: 0x0D,
        Key.Space: 0x20,
        # Modifier Keys
        Key.Ctrl: 0x11,  # VK_CONTROL
        Key.Shift: 0x10,  # VK_SHIFT
        Key.Alt: 0x12,  # VK_MENU
        # Punctuation & Special Characters
        Key.Tilde: 0xC0,  # `
        Key.Quote: 0xDE,  # '
        Key.Semicolon: 0xBA,  # ;
        Key.Comma: 0xBC,  # ,
        Key.Period: 0xBE,  # .
        Key.Slash: 0xBF,  # /
    }

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=1))
    add_KeyInputServicer_to_server(KeyInput(window, keys_map, vk_keys_map), server)
    server.add_insecure_port("[::]:5001")
    server.start()
    print("Server started, listening on 5001")
    server.wait_for_termination()
