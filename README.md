# AMDGPU Settings (Rust)

CLI tool to monitor and tune AMDGPU settings.

Improved version of [amdgpu-settings](https://github.com/yuheho7749/amdgpu-settings) rewritten in Rust.

## Features
- Supports RDNA 2 and 3. RDNA 4 is unsupported at the moment.
- `Systemd` service to automatically apply profile on startup.
- Support multiple GPU profiles.

*Note: This is only developed and tested using a RDNA 3 GPU.*

## Prerequisites
- Linux kernel 6.13 or newer (required for `FAN_ZERO_RPM_ENABLE` and `FAN_ZERO_RPM_STOP_TEMPERATURE` settings) OR Linux kernel 6.10 or newer.
- Kernel parameters must be set according to [this](https://wiki.archlinux.org/title/AMDGPU#Boot_parameter).
- Cargo/rustc.

## Installation
1. Run `cargo build --release`.
2. Run the `install.sh` script to install files.
3. `cp` the `amdgpu-settings.example` profile to `/etc/default/amdgpu-settings.[PROFILE_NAME]`. It is HIGHLY recommended to have `/etc/default/amdgpu-settings.default` as it will be the profile used by default. *Tip: Use a symlink `/etc/default/amdgpu-settings.default` to point to another profile.*

### Optional `Systemd` Installation
- For auto-start, enable the service with `systemctl enable amdgpu-settings`. NOTE: `/etc/default/amdgpu-settings.default` must exist as the service will use that profile by default. *Tip: You can use a symlink to avoid editing the `amdgpu-settings.service` file.*

## Usage
- `amdgpu-settings set [PROFILE_NAME]` to apply profile settings (require elevated privileges).
- `amdgpu-settings reset [PROFILE_NAME]` to reset card# specified by the profile (require elevated privileges).
- `amdgpu-settings info [PROFILE_NAME]` to read card# settings specified by the profile.
- `amdgpu-settings --help`.

## GPU Profile Format
The profile **MUST** have `CARD: #` as the first line. That will be used to find where the GPU is mounted in the file system. To check which card number your GPU is mounted at, navigate to `/sys/class/drm/`. The GPU will most likely be mounted as `card0` or `card1`, although it may vary from system to system.

The currently supported options are:
- `OD_SCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_MCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_VDDGFX_OFFSET`
- `POWER_CAP`
- `FAN_TARGET_TEMPERATURE`
- `FAN_ZERO_RPM_ENABLE` (Only available on Linux 6.13 or newer)
- `FAN_ZERO_RPM_STOP_TEMPERATURE` (Only available on Linux 6.13 or newer)

An example of a GPU profile is shown below:
```
CARD: 1

OD_SCLK:
1: 2500Mhz

OD_VDDGFX_OFFSET:
-150mV

POWER_CAP:
240000000

FAN_TARGET_TEMPERATURE:
80

FAN_ZERO_RPM_ENABLE:
1

FAN_ZERO_RPM_STOP_TEMPERATURE:
50
```
