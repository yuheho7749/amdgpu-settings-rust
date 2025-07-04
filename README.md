# AMDGPU Settings (Rust)

CLI tool written in Rust to monitor and tune AMDGPU settings.

Inspired by [amdgpu-clocks](https://github.com/sibradzic/amdgpu-clocks).

## Features
- Supports RDNA 3 and RDNA 4 (at least Linux 6.12 for RDNA 4). RDNA 2 or older might work, but is untested.
- `Systemd` service to automatically apply profile on startup.
- Support multiple GPU profiles.

> [!WARNING]
> Development and testing has migrated to RDNA 4 platform. RDNA 3 testing is deprecated.

## Prerequisites
- Linux kernel 6.10 or newer.
- Linux kernel 6.13 or newer for `FAN_ZERO_RPM_ENABLE` and `FAN_ZERO_RPM_STOP_TEMPERATURE` settings.
- Linux kernel 6.12 or newer for RDNA 4 (Linux 6.14 is recommended).
- Kernel parameters must be set according to [this](https://wiki.archlinux.org/title/AMDGPU#Boot_parameter).
- Cargo

## Installation
1. Run the `install.sh` script to build and install files.
2. `cp` the `amdgpu-settings.example` profile to `/etc/default/amdgpu-settings.[PROFILE_NAME]`. It is HIGHLY recommended to have `/etc/default/amdgpu-settings.default` as it will be the profile used by default.

> [!TIP]
> Use a symlink to set the default profile `/etc/default/amdgpu-settings.default`. Additionally, you define multiple profiles `/etc/default/amdgpu-settings.[PROFILE_NAME]` and quickly swap profiles with `sudo amdgpu-settings set [PROFILE_NAME]`

### Optional `Systemd` Installation
- For auto-start, enable the service with `systemctl enable amdgpu-settings`

> [!IMPORTANT]
> `/etc/default/amdgpu-settings.default` must exist as the service will use that profile by default. You should use a symlink to avoid editing the `amdgpu-settings.service` file.

> [!NOTE]
> Suspending and resuming your system will reset to the default profile (reloads the systemctl service to the default state). To remove this behavior, remove the file `/usr/lib/systemd/system-sleep/amdgpu-settings.resume` (after running the `install.sh` script) or modify the `install.sh` script before installation.

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
- `OD_SCLK_OFFSET` RDNA 4 specific setting
- `OD_SCLK` RDNA 3 or older specific setting (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_MCLK` (the '0: [value]Mhz' means min frequency and '1: [value]Mhz' means max frequency)
- `OD_VDDGFX_OFFSET` GPU core voltage offset
- `POWER_CAP`
- `FAN_TARGET_TEMPERATURE`
- `FAN_ZERO_RPM_ENABLE` (Only available on Linux 6.13 or newer)
- `FAN_ZERO_RPM_STOP_TEMPERATURE` (Only available on Linux 6.13 or newer)

### RDNA 3 or older
An example of a RDNA 3 GPU profile is shown below:
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

### RDNA 4
An example of a RDNA 4 GPU profile is shown below:
```
CARD: 1

PERFORMANCE_LEVEL:
manual

POWER_PROFILE_INDEX:
0

OD_SCLK_OFFSET:
-100Mhz

OD_VDDGFX_OFFSET:
-50mV

POWER_CAP:
290000000

FAN_TARGET_TEMPERATURE:
80

FAN_ZERO_RPM_ENABLE:
1

FAN_ZERO_RPM_STOP_TEMPERATURE:
50
```
