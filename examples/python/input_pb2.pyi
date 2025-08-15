from google.protobuf.internal import enum_type_wrapper as _enum_type_wrapper
from google.protobuf import descriptor as _descriptor
from google.protobuf import message as _message
from typing import ClassVar as _ClassVar, Optional as _Optional, Union as _Union

DESCRIPTOR: _descriptor.FileDescriptor

class Key(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    A: _ClassVar[Key]
    B: _ClassVar[Key]
    C: _ClassVar[Key]
    D: _ClassVar[Key]
    E: _ClassVar[Key]
    F: _ClassVar[Key]
    G: _ClassVar[Key]
    H: _ClassVar[Key]
    I: _ClassVar[Key]
    J: _ClassVar[Key]
    K: _ClassVar[Key]
    L: _ClassVar[Key]
    M: _ClassVar[Key]
    N: _ClassVar[Key]
    O: _ClassVar[Key]
    P: _ClassVar[Key]
    Q: _ClassVar[Key]
    R: _ClassVar[Key]
    S: _ClassVar[Key]
    T: _ClassVar[Key]
    U: _ClassVar[Key]
    V: _ClassVar[Key]
    W: _ClassVar[Key]
    X: _ClassVar[Key]
    Y: _ClassVar[Key]
    Z: _ClassVar[Key]
    Zero: _ClassVar[Key]
    One: _ClassVar[Key]
    Two: _ClassVar[Key]
    Three: _ClassVar[Key]
    Four: _ClassVar[Key]
    Five: _ClassVar[Key]
    Six: _ClassVar[Key]
    Seven: _ClassVar[Key]
    Eight: _ClassVar[Key]
    Nine: _ClassVar[Key]
    F1: _ClassVar[Key]
    F2: _ClassVar[Key]
    F3: _ClassVar[Key]
    F4: _ClassVar[Key]
    F5: _ClassVar[Key]
    F6: _ClassVar[Key]
    F7: _ClassVar[Key]
    F8: _ClassVar[Key]
    F9: _ClassVar[Key]
    F10: _ClassVar[Key]
    F11: _ClassVar[Key]
    F12: _ClassVar[Key]
    Up: _ClassVar[Key]
    Down: _ClassVar[Key]
    Left: _ClassVar[Key]
    Right: _ClassVar[Key]
    Home: _ClassVar[Key]
    End: _ClassVar[Key]
    PageUp: _ClassVar[Key]
    PageDown: _ClassVar[Key]
    Insert: _ClassVar[Key]
    Delete: _ClassVar[Key]
    Ctrl: _ClassVar[Key]
    Enter: _ClassVar[Key]
    Space: _ClassVar[Key]
    Tilde: _ClassVar[Key]
    Quote: _ClassVar[Key]
    Semicolon: _ClassVar[Key]
    Comma: _ClassVar[Key]
    Period: _ClassVar[Key]
    Slash: _ClassVar[Key]
    Esc: _ClassVar[Key]
    Shift: _ClassVar[Key]
    Alt: _ClassVar[Key]

class KeyState(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    Pressed: _ClassVar[KeyState]
    Released: _ClassVar[KeyState]

class MouseAction(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    Move: _ClassVar[MouseAction]
    Click: _ClassVar[MouseAction]
    ScrollDown: _ClassVar[MouseAction]

class Coordinate(int, metaclass=_enum_type_wrapper.EnumTypeWrapper):
    __slots__ = ()
    Screen: _ClassVar[Coordinate]
    Relative: _ClassVar[Coordinate]
A: Key
B: Key
C: Key
D: Key
E: Key
F: Key
G: Key
H: Key
I: Key
J: Key
K: Key
L: Key
M: Key
N: Key
O: Key
P: Key
Q: Key
R: Key
S: Key
T: Key
U: Key
V: Key
W: Key
X: Key
Y: Key
Z: Key
Zero: Key
One: Key
Two: Key
Three: Key
Four: Key
Five: Key
Six: Key
Seven: Key
Eight: Key
Nine: Key
F1: Key
F2: Key
F3: Key
F4: Key
F5: Key
F6: Key
F7: Key
F8: Key
F9: Key
F10: Key
F11: Key
F12: Key
Up: Key
Down: Key
Left: Key
Right: Key
Home: Key
End: Key
PageUp: Key
PageDown: Key
Insert: Key
Delete: Key
Ctrl: Key
Enter: Key
Space: Key
Tilde: Key
Quote: Key
Semicolon: Key
Comma: Key
Period: Key
Slash: Key
Esc: Key
Shift: Key
Alt: Key
Pressed: KeyState
Released: KeyState
Move: MouseAction
Click: MouseAction
ScrollDown: MouseAction
Screen: Coordinate
Relative: Coordinate

class KeyInitRequest(_message.Message):
    __slots__ = ("seed",)
    SEED_FIELD_NUMBER: _ClassVar[int]
    seed: bytes
    def __init__(self, seed: _Optional[bytes] = ...) -> None: ...

class KeyInitResponse(_message.Message):
    __slots__ = ("mouse_coordinate",)
    MOUSE_COORDINATE_FIELD_NUMBER: _ClassVar[int]
    mouse_coordinate: Coordinate
    def __init__(self, mouse_coordinate: _Optional[_Union[Coordinate, str]] = ...) -> None: ...

class KeyStateRequest(_message.Message):
    __slots__ = ("key",)
    KEY_FIELD_NUMBER: _ClassVar[int]
    key: Key
    def __init__(self, key: _Optional[_Union[Key, str]] = ...) -> None: ...

class KeyStateResponse(_message.Message):
    __slots__ = ("state",)
    STATE_FIELD_NUMBER: _ClassVar[int]
    state: KeyState
    def __init__(self, state: _Optional[_Union[KeyState, str]] = ...) -> None: ...

class MouseRequest(_message.Message):
    __slots__ = ("width", "height", "x", "y", "action")
    WIDTH_FIELD_NUMBER: _ClassVar[int]
    HEIGHT_FIELD_NUMBER: _ClassVar[int]
    X_FIELD_NUMBER: _ClassVar[int]
    Y_FIELD_NUMBER: _ClassVar[int]
    ACTION_FIELD_NUMBER: _ClassVar[int]
    width: int
    height: int
    x: int
    y: int
    action: MouseAction
    def __init__(self, width: _Optional[int] = ..., height: _Optional[int] = ..., x: _Optional[int] = ..., y: _Optional[int] = ..., action: _Optional[_Union[MouseAction, str]] = ...) -> None: ...

class MouseResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class KeyRequest(_message.Message):
    __slots__ = ("key", "down_ms")
    KEY_FIELD_NUMBER: _ClassVar[int]
    DOWN_MS_FIELD_NUMBER: _ClassVar[int]
    key: Key
    down_ms: float
    def __init__(self, key: _Optional[_Union[Key, str]] = ..., down_ms: _Optional[float] = ...) -> None: ...

class KeyResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class KeyDownRequest(_message.Message):
    __slots__ = ("key",)
    KEY_FIELD_NUMBER: _ClassVar[int]
    key: Key
    def __init__(self, key: _Optional[_Union[Key, str]] = ...) -> None: ...

class KeyDownResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...

class KeyUpRequest(_message.Message):
    __slots__ = ("key",)
    KEY_FIELD_NUMBER: _ClassVar[int]
    key: Key
    def __init__(self, key: _Optional[_Union[Key, str]] = ...) -> None: ...

class KeyUpResponse(_message.Message):
    __slots__ = ()
    def __init__(self) -> None: ...
