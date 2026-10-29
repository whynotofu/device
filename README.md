## Device Daemon

Battery, Display Brightness, Keyboard Backlight and Platform Profile Daemon with presistence.

### Battery Protection

| Mode       | Start Threshold | End Threshold |
| ---------- | --------------- | ------------- |
| Off        | 95%             | 100%          |
| On         | 75%             | 80%           |
| Stationary | 40%             | 60%           |

### Battery Start Threshold Emulation

On devices that only expose end threshold - start threshold is emulated by lowering end threshold when upper limit is reached and raising it when lower limit is reached.

Limitation: If upper or lower threshold is reached when suspended, firmware will keep topping up battery to: a) start threshold - 5%; b) min(end threshold + 5%, 100).

### API

Rust tokyo-stream ReceiverStream based api in device-service crate.

### CLI

```
device-cli get -h
device-cli set -h
```

## Config

Location: `/etc/device.toml`

| Option | Description | Type | Default |
| --- | --- | --- | --- |
| keyboard_backlight | If not set, first `{name}::kbd_backlight` encountered in `/sys/class/leds/` will be used. | String | |
| battery_polling_interval | 5, 10, 15, 20, 30 or 60 seconds. | Integer | 10 |

### Static Config Section

| Option | Description | Type | Default |
| --- | --- | --- | --- |
| intel_turbo | Enable/Disable Intel turbo. | Boolean | true |

Example:

```
[static_config]
intel_turbo = false
```

## Install

### Fedora

```
sudo dnf copr enable whynotofu/device
sudo dnf install device
sudo systemctl enable device
sudo systemctl start device
```
