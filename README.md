# AMDGPU Settings (Rust)

Cli tool to monitor and set AMDGPU settings.

Improved version of [amdgpu-settings](https://github.com/yuheho7749/amdgpu-settings) rewritten in Rust.

## Features
- `Systemd` service to automatically apply profile on startup.
- Support multiple GPU profiles.

Note: This is only developed and tested on Linux 6.12+ using RDNA 3 GPU.

## Prerequisites
- Linux kernel 6.10 or up (lts 6.6 will not work).
- Kernel parameters must be set according to [this](https://wiki.archlinux.org/title/AMDGPU#Boot_parameter).
- Cargo/rustc.

## Installation
1. Run `cargo build --release`.
2. Run the `install.sh` script to install file.
3. `cp` the `amdgpu-settings.example` profile to `/etc/default/amdgpu-settings.[PROFILE-NAME]`.

### Optional `Systemd` Installation
- For auto-start, enable the service with `systemctl enable amdgpu-settings`

## Usage
- `amdgpu-settings set [PROFILE-NAME]` to apply profile settings (require elevated priviledges).
- `amdgpu-settings reset [CARD-NUM]` to reset device/card settings (require elevated priviledges).
- `amdgpu-settings info [CARD-NUM]` to read device/card settings.
- `amdgpu-settings --help`

## GPU Profile Format
The profile **MUST** have `CARD: #` as the first line. That will be used to find where the GPU is mounted in the file system. To check which card number your GPU is mounted at, navigate to `/sys/class/drm/`. The GPU will most likely be mounted as `card0` or `card1`, although it may vary from system to system.

The currently supported options are:
- `OD_SCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_MCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_VDDGFX_OFFSET`
- `POWER_CAP`

An example of a GPU profile is shown below:
```
CARD: 1

OD_SCLK:
1: 2500Mhz

OD_VDDGFX_OFFSET:
-150mV

POWER_CAP:
240000000
```
