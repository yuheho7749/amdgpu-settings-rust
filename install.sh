#!/usr/bin/env bash

cp ./target/release/amdgpu-settings /usr/local/bin/amdgpu-settings
chmod +x /usr/local/bin/amdgpu-settings

cp ./amdgpu-settings.resume /usr/lib/systemd/system-sleep/amdgpu-settings.resume
chmod +x /usr/lib/systemd/system-sleep/amdgpu-settings.resume

cp ./amdgpu-settings.service /etc/systemd/system/amdgpu-settings.service
