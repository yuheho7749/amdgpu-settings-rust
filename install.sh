#!/usr/bin/env bash

cargo build --release

sudo cp ./target/release/amdgpu-settings /usr/local/bin/amdgpu-settings
sudo chmod +x /usr/local/bin/amdgpu-settings

sudo cp ./src/amdgpu-settings.resume /usr/lib/systemd/system-sleep/amdgpu-settings.resume
sudo chmod +x /usr/lib/systemd/system-sleep/amdgpu-settings.resume

sudo cp ./src/amdgpu-settings.service /etc/systemd/system/amdgpu-settings.service
