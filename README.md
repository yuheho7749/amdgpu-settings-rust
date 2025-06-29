# AMDGPU Settings (Rust)

CLI tool to monitor and tune AMDGPU settings.

Improved version of [amdgpu-settings](https://github.com/yuheho7749/amdgpu-settings) rewritten in Rust.

## Features
- Supports RDNA 3. RDNA 2 or older might work, but is untested. RDNA 4 is unsupported at the moment.
- `Systemd` service to automatically apply profile on startup.
- Support multiple GPU profiles.

> [!NOTE]
> This is only developed and tested using a RDNA 3 GPU.

## Prerequisites
- Linux kernel 6.13 or newer (required for `FAN_ZERO_RPM_ENABLE` and `FAN_ZERO_RPM_STOP_TEMPERATURE` settings) OR Linux kernel 6.10 or newer.
- Kernel parameters must be set according to [this](https://wiki.archlinux.org/title/AMDGPU#Boot_parameter).
- Cargo

## Installation
1. Run the `install.sh` script to build and install files.
2. `cp` the `amdgpu-settings.example` profile to `/etc/default/amdgpu-settings.[PROFILE_NAME]`. It is HIGHLY recommended to have `/etc/default/amdgpu-settings.default` as it will be the profile used by default.

> [!TIP]
> Use a symlink to set the default profile set by systemd (`/etc/default/amdgpu-settings.default`)

### Optional `Systemd` Installation
- For auto-start, enable the service with `systemctl enable amdgpu-settings`. NOTE: `/etc/default/amdgpu-settings.default` must exist as the service will use that profile by default. *Tip: You can use a symlink to avoid editing the `amdgpu-settings.service` file.*

## Usage
- `amdgpu-settings set [PROFILE_NAME]` to reset and apply new profile settings (require elevated/sudo privileges).
- `amdgpu-settings reset [PROFILE_NAME]` to reset card# specified by the profile (require elevated/sudo privileges).
- `amdgpu-settings info [PROFILE_NAME]` to read card# settings specified by the profile.
- `amdgpu-settings --help`.

## GPU Profile Format
The profile **MUST** have `CARD: #` as the first line. That will be used to find where the GPU is mounted in the file system. To check which card number your GPU is mounted at, navigate to `/sys/class/drm/`. The GPU will most likely be mounted as `card0` or `card1`, although it may vary from system to system.

The currently supported options are:
- `PERFORMANCE_LEVEL` Unless specified, applying a new profile will default to the `manual` [performance level](https://wiki.archlinux.org/title/AMDGPU#Performance_levels).
- `POWER_PROFILE_INDEX` ([Power profiles](https://wiki.archlinux.org/title/AMDGPU#Power_profiles): e.g. BOOTUP_DEFAULT, 3D_FULL_SCREEN, COMPUTE, etc)
- `OD_SCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_MCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_VDDGFX_OFFSET` GPU core voltage offset
- `POWER_CAP`
- `FAN_TARGET_TEMPERATURE`
- `FAN_ZERO_RPM_ENABLE` (Only available on Linux 6.13 or newer)
- `FAN_ZERO_RPM_STOP_TEMPERATURE` (Only available on Linux 6.13 or newer)

An example of a GPU profile is shown below:
```
CARD: 1

PERFORMANCE_LEVEL:
manual

POWER_PROFILE_INDEX:
0

OD_SCLK:
1: 2500Mhz

OD_VDDGFX_OFFSET:
-100mV

POWER_CAP:
240000000

FAN_TARGET_TEMPERATURE:
80

FAN_ZERO_RPM_ENABLE:
1

FAN_ZERO_RPM_STOP_TEMPERATURE:
50
```
