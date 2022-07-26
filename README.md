andrgesture
---

The program I use on rooted Pixel 3 XL with Android 12 to control the torch.
Combined with [torchctl](https://github.com/vi/torchctl) it allows to adjust torch brightness using special sequence of inputs,
without iteraction with usual Android userland.

Probably the task is attainable with Android programming as well, but I know evdev better than Android, so implemented it that way.
And it's unlikely that proper Android implementation would be more lightweight with the same level or reliability.

There is a published pre-built executable on Github releases.

Gesture
---

Press power button, then interact with specific spot of touchscreen, doing clockwise or counterclockwise movements.
After 3 clockwise movements, the torch turns on in minimal mode, each subsequenct turn increses the brightness.
After 2 counterclockwise movements the torch gets dimmer or gets turned off.

Obviously, the power button should wake up screen, not shut it down.

The program listens either keyboard (i.e. power/voldn buttons) or touchscreen events, not both. Timeouts are used to manage that attention.


Usage
---

Insect the help message and experiment around. This project is published mostly for backup purposes, not to promote usage of such hacks.

With overridden parameters the program may also be used on Desktop Linux as well.
