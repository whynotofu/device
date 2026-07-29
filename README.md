# Device Daemon

## Feature Matrix

| Feature            | Get | Set | Subscribe | Persistence |
| ------------------ | --- | --- | --------- | ----------- |
| Battery Charge     | Y   |     | Y         |             |
| Battery State      | Y   |     | Y         |             |
| Battery Protection | Y   | Y   | Y         | Y           |
| Display Brightness | Y   | Y   | Y         | Y           |
| Keyboard Backlight | Y   | Y   | Y         | Y           |
| Platform Profile   | Y   | Y   | Y         | Y           |

## Battery Protection

| Mode       | Start Threshold | End Threshold |
| ---------- | --------------- | ------------- |
| Off        | 95%             | 100%          |
| On         | 75%             | 80%           |
| Stationary | 40%             | 60%           |

## Battery Start Threshold Emulation

On devices that only expose end threshold - start threshold is emulated by lowering end threshold when upper limit is reached and raising it when lower limit is reached.

Limitation: If upper or lower threshold is reached when suspended, firmware will keep topping up battery to that threshold.

## Config

Location: `/etc/device.toml`

| Option | Description | Default |
| --- | --- | --- |
| keyboard_backlight | Name as it appears in `/sys/class/leds/{name}::kbd_backlight`. Setting this option preempts autoselect. | |
| battery_polling_interval | 5, 10, 15, 20, 30 or 60 seconds. | 10 |

### Static Config Section

| Option | Description | Default |
| --- | --- | --- |
| intel_turbo | Enable/Disable Intel turbo. | true |

Example:

```
[static_config]
intel_turbo = false
```

## CLI

`device-cli help`
