// SPDX-License-Identifier: GPL-2.0-only

/*
 * CLI tool for AMD RDNA GPUs
 *
 * Copyright (c) 2025 yuheho7749
 */

use std::fs;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use std::io::{BufRead, BufReader, Write};
use clap::{Parser, Subcommand};
use glob::glob;

const CONFIG_PROFILE_PATH: &str = "/etc/default/amdgpu-settings.";

#[derive(Default, Debug)]
struct DeviceConfig {
    device_id: Option<u64>,
    card: Option<u8>,
    home_path: PathBuf,
    hwmon_path: PathBuf,
    performance_level: Option<String>,
    power_profile_index: Option<u8>,
    od_sclk_min: Option<u32>, // RDNA 2 and 3
    od_sclk_max: Option<u32>, // RDNA 2 and 3
    od_sclk_offset: Option<i32>, // RDNA 4
    od_mclk_min: Option<u32>,
    od_mclk_max: Option<u32>,
    od_vddgfx_offset: Option<i32>,
    power_cap: Option<u64>,
    fan_target_temp: Option<u32>,
    fan_zero_rpm: Option<u8>,
    fan_zero_rpm_stop_temp: Option<u32>,
}

fn validate_detect_mount_points(config: &mut DeviceConfig) {
    let path: PathBuf = if let Some(card) = config.card {
        // Simple card #
        PathBuf::from(format!("/sys/class/drm/card{}", card))
    } else {
        // Match by unique_id
        let unique_id = config.device_id.expect("Invalid UNIQUE_ID");

        let mut found_path: Option<PathBuf> = None;
        for entry in glob("/sys/class/drm/card*").expect("Failed to detect drm card path") {
            if let Ok(card_path) = entry {
                let unique_id_path = card_path.join("device/unique_id");
                if let Ok(target_id_str) = fs::read_to_string(unique_id_path) {
                    let target_id = u64::from_str_radix(target_id_str.trim(), 16)
                        .expect("Malformed unique_id");
                    if target_id == unique_id {
                        found_path = Some(card_path);
                        break;
                    }
                }
            }
        }
        found_path.expect("Fatal error: Unable to locate card mount point by unique_id. Please check /sys/class/drm")
    };

    config.home_path = path.join("device");
    let hwmon_pattern = path.join("device/hwmon/hwmon*");
    for entry in glob(hwmon_pattern.to_str().unwrap()).expect("Failed to detect hwmon path") {
        if let Ok(hwmon_path) = entry {
            config.hwmon_path = hwmon_path;
            return;
        }
    }
    panic!("Unable to detect hwmon_path!!!");
}


fn parse_performance_level(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid PERFORMANCE_LEVEL");
    config.performance_level = Some(value);
}

fn parse_power_profile_index(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid POWER_PROFILE_INDEX");
    config.power_profile_index = Some(value);
}

// RDNA 4 core clk speed
fn parse_od_sclk_offset(config: &mut DeviceConfig, lines: &[String]) {
    let sclk_offset: i32 = (&lines[1].split("M").collect::<Vec<&str>>()[0])
        .parse().expect("Invalid OD_SCLK_OFFSET option");
    config.od_sclk_offset = Some(sclk_offset);
}

// RDNA 3 or older clk speed
fn parse_od_sclk(config: &mut DeviceConfig, lines: &[String]) {
    let mut i: usize = 1;

    while lines[i].len() != 0 {
        let sclk: (char, u32) = (
            lines[i].chars().nth(0).expect("Invalid OD_SCLK option"),
            (&lines[i][3..].split("M").collect::<Vec<&str>>()[0])
                .parse().expect("Invalid OD_SCLK option")
        );
        match sclk.0 {
            '0' => config.od_sclk_min = Some(sclk.1),
            '1' => config.od_sclk_max = Some(sclk.1),
            _ => {
                println!("Invalid OD_SCLK option");
                return;
            },
        }
        i += 1;
    }
}

fn parse_od_mclk(config: &mut DeviceConfig, lines: &[String]) {
    let mut i: usize = 1;

    while lines[i].len() != 0 {
        let mclk: (char, u32) = (
            lines[i].chars().nth(0).expect("Invalid OD_MCLK option"),
            (&lines[i][3..].split("M").collect::<Vec<&str>>()[0])
                .parse().expect("Invalid OD_MCLK option")
        );
        match mclk.0 {
            '0' => config.od_mclk_min = Some(mclk.1),
            '1' => config.od_mclk_max = Some(mclk.1),
            _ => {
                println!("Invalid OD_MCLK option");
                return;
            },
        }
        i += 1;
    }
}

fn parse_od_vddgfx_offset(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].split("m")
        .collect::<Vec<&str>>()[0]
        .parse().expect("Invalid voltage");
    config.od_vddgfx_offset = Some(value);
}

fn parse_power_cap(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid POWER_CAP");
    config.power_cap = Some(value);
}

fn parse_fan_target_temp(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid FAN_TARGET_TEMPERATURE");
    config.fan_target_temp = Some(value);
}

fn parse_fan_zero_rpm(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid FAN_ZERO_RPM_ENABLE");
    config.fan_zero_rpm = Some(value);
}

fn parse_fan_zero_rpm_stop_temp(config: &mut DeviceConfig, lines: &[String]) {
    let value = lines[1].parse().expect("Invalid FAN_ZERO_RPM_STOP_TEMPERATURE");
    config.fan_zero_rpm_stop_temp = Some(value);
}

fn parse_profile(path: &str) -> DeviceConfig {
    let file = File::open(path).expect("Profile not found");
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let id_str: &str = &lines[0];
    let (id_type, id) = id_str.split_once(char::is_whitespace)
        .expect("Error parsing CARD/UNIQUE_ID");
    let mut config = DeviceConfig::default();
    match id_type {
        "CARD:" => config.card = Some(id.trim().parse().expect("Invalid CARD #")),
        "UNIQUE_ID:" => config.device_id = Some(u64::from_str_radix(id.trim(), 16).expect("Invalid UNIQUE_ID #")),
        _ => panic!("Unknown target device: Check /sys/class/drm"),
    }
    validate_detect_mount_points(&mut config);

    let mut i: usize = 0;
    while i < lines.len() {
        let line: &str = &lines[i].trim();
        match line {
            "PERFORMANCE_LEVEL:" => parse_performance_level(&mut config, &lines[i..]),
            "POWER_PROFILE_INDEX:" => parse_power_profile_index(&mut config, &lines[i..]),
            "OD_SCLK_OFFSET:" => parse_od_sclk_offset(&mut config, &lines[i..]),
            "OD_SCLK:" => parse_od_sclk(&mut config, &lines[i..]),
            "OD_MCLK:" => parse_od_mclk(&mut config, &lines[i..]),
            "OD_VDDGFX_OFFSET:" => parse_od_vddgfx_offset(&mut config, &lines[i..]),
            "POWER_CAP:" => parse_power_cap(&mut config, &lines[i..]),
            "FAN_TARGET_TEMPERATURE:" => parse_fan_target_temp(&mut config, &lines[i..]),
            "FAN_ZERO_RPM_ENABLE:" => parse_fan_zero_rpm(&mut config, &lines[i..]),
            "FAN_ZERO_RPM_STOP_TEMPERATURE:" => parse_fan_zero_rpm_stop_temp(&mut config, &lines[i..]),
            _ => {}
        }
        i += 1;
    }
    return config;
}

fn apply_settings(name: &str, mut config: DeviceConfig) {
    validate_detect_mount_points(&mut config);
    println!("---------- {} Settings ----------", name.to_uppercase());
    println!("{:#?}", config);

    // PERFORMANCE_LEVEL
    let mut file = OpenOptions::new().write(true)
        .open(config.home_path.join("power_dpm_force_performance_level"))
        .expect("Can't access power_dpm_force_performance_level file");
    if config.performance_level.is_some() {
        file.write_all(config.performance_level.as_deref().unwrap_or("manual").as_bytes())
            .expect("Failed to write power_dpm_force_performance_level");
    } else {
        file.write_all("manual".as_bytes())
            .expect("Failed to write power_dpm_force_performance_level to \"manual\"");
    }

    // POWER_CAP (Side effect of writing a new value will reset GPU settings. Should set this
    // before adjusting the other settings)
    if config.power_cap.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(config.hwmon_path.join("power1_cap"))
            .expect("Can't access power1_cap file");
        file.write_all(config.power_cap.unwrap().to_string().as_bytes())
            .expect("Failed to set POWER_CAP");
    }

    // POWER_PROFILE_INDEX
    if config.power_profile_index.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(config.home_path.join("pp_power_profile_mode"))
            .expect("Can't access pp_power_profile_mode file");
        file.write_all(config.power_profile_index.unwrap().to_string().as_bytes())
            .expect("Failed to set POWER_PROFILE_INDEX");
    }

    // FAN_TARGET_TEMPERATURE 
    if config.fan_target_temp.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(config.home_path.join("gpu_od/fan_ctrl/fan_target_temperature"))
            .expect("Can't access fan_target_temperature file");
        file.write_all(format!("{}\n", config.fan_target_temp.unwrap()).as_bytes())
            .expect("Failed to write FAN_TARGET_TEMPERATURE");
    }
    // FAN_ZERO_RPM_ENABLE
    if config.fan_zero_rpm.is_some() {
        let file_result = OpenOptions::new().write(true)
            .open(config.home_path.join("gpu_od/fan_ctrl/fan_zero_rpm_enable"));
        if let Ok(mut file) = file_result {
            file.write_all(format!("{}\n", config.fan_zero_rpm.unwrap()).as_bytes())
                .expect("Failed to write FAN_ZERO_RPM_ENABLE");
        } else {
            println!("Skip setting FAN_ZERO_RPM_ENABLE. Make sure to have Linux 6.13 or newer.");
        }
    }
    // FAN_ZERO_RPM_STOP_TEMPERATURE
    if config.fan_zero_rpm_stop_temp.is_some() {
        let file_result = OpenOptions::new().write(true)
            .open(config.home_path.join("gpu_od/fan_ctrl/fan_zero_rpm_stop_temperature"));
        if let Ok(mut file) = file_result {
            file.write_all(format!("{}\n", config.fan_zero_rpm_stop_temp.unwrap()).as_bytes())
                .expect("Failed to write FAN_ZERO_RPM_STOP_TEMPERATURE");
        } else {
            println!("Skip setting FAN_ZERO_RPM_STOP_TEMPERATURE. Make sure to have Linux 6.13 or newer.");
        }
    }

    // pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(config.home_path.join("pp_od_clk_voltage"))
        .expect("Can't access pp_od_clk_voltage file");

    // OD_SCLK_OFFSET (RDNA 4)
    if config.od_sclk_offset.is_some() {
        if config.od_sclk_offset.is_some() {
            file.write_all(format!("s {}", config.od_sclk_offset.unwrap()).as_bytes())
                .expect("Failed to write od_sclk_offset");
        }
    } else { // OD_SCLK (RDNA 3 or older)
        if config.od_sclk_min.is_some() {
            file.write_all(format!("s 0 {}", config.od_sclk_min.unwrap()).as_bytes())
                .expect("Failed to write od_sclk_min");
        }
        if config.od_sclk_max.is_some() {
            file.write_all(format!("s 1 {}", config.od_sclk_max.unwrap()).as_bytes())
                .expect("Failed to write od_sclk_max");
        }
    }
    // OD_MCLK
    if config.od_mclk_min.is_some() {
        file.write_all(format!("m 0 {}", config.od_mclk_min.unwrap()).as_bytes())
            .expect("Failed to write od_mclk_min");
    }
    if config.od_mclk_max.is_some() {
        file.write_all(format!("m 1 {}", config.od_mclk_max.unwrap()).as_bytes())
            .expect("Failed to write od_mclk_max");
    }
    // OD_VDDGFX_OFFSET
    if config.od_vddgfx_offset.is_some() {
        file.write_all(format!("vo {}", config.od_vddgfx_offset.unwrap()).as_bytes())
            .expect("Failed to write od_vddgfx_offset");
    }
    // NOTE: Commit to pp_od_clk_voltage (but it will actually just commit all "committable" settings on at least RDNA 3 or newer)
    // By "committable", see https://docs.kernel.org/gpu/amdgpu/thermal.html for all settings that require an explicit "c" to commit
    file.write_all("c".as_bytes())
        .expect("Failed to commit final settings");
    println!("Success!");
}

fn reset_settings(path: &str) {
    let file = File::open(path).expect("Profile not found");
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let id_str: &str = &lines[0];
    let (id_type, id) = id_str.split_once(char::is_whitespace)
        .expect("Error parsing CARD/UNIQUE_ID");
    let mut config = DeviceConfig::default();
    match id_type {
        "CARD:" => config.card = Some(id.trim().parse().expect("Invalid CARD #")),
        "UNIQUE_ID:" => config.device_id = Some(u64::from_str_radix(id.trim(), 16).expect("Invalid UNIQUE_ID #")),
        _ => panic!("Unknown target device: Check /sys/class/drm"),
    }
    validate_detect_mount_points(&mut config);

    if config.card.is_some() {
        println!("Resetting card {}...", config.card.unwrap());
    } else if config.device_id.is_some() {
        println!("Resetting device {:x}...", config.device_id.unwrap());
    }

    // Reset PERFORMANCE_LEVEL
    let mut file = OpenOptions::new().write(true)
        .open(config.home_path.join("power_dpm_force_performance_level"))
        .expect("Can't access power_dpm_force_performance_level file");
    file.write_all("auto".as_bytes())
        .expect("Failed to reset power_dpm_force_performance_level to \"auto\"");

    // Reset POWER_CAP
    let file = File::open(config.hwmon_path.join("power1_cap_default"))
        .expect("Can't access power1_cap file");
    let power_cap_default_lines: Vec<String> = BufReader::new(file).lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let power_cap_default = &power_cap_default_lines[0];
    let mut file = OpenOptions::new().write(true)
        .open(config.hwmon_path.join("power1_cap"))
        .expect("Can't access power1_cap file");
    file.write_all(power_cap_default.as_bytes())
        .expect("Failed to reset POWER_CAP");

    // Reset POWER_PROFILE_INDEX
    let mut file = OpenOptions::new().write(true)
        .open(config.home_path.join("pp_power_profile_mode"))
        .expect("Can't access pp_od_clk_voltage file");
    file.write_all("0".as_bytes()) // 0 is BOOTUP_DEFAULT
        .expect("Failed to reset power profile to BOOTUP_DEFAULT");

    // Reset pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(config.home_path.join("pp_od_clk_voltage"))
        .expect("Can't access pp_od_clk_voltage file");
    file.write_all("r".as_bytes())
        .expect("Failed to reset card with pp_od_clk_voltage_file");

    // NOTE: AMDGPU driver also resets every settings that is "committable".
    // By "committable", see https://docs.kernel.org/gpu/amdgpu/thermal.html for all settings that require an explicit "c" to commit


    println!("Success!");
}

fn read_card_settings(path: &str) {
    let file = File::open(path).expect("Profile not found");
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let id_str: &str = &lines[0];
    let (id_type, id) = id_str.split_once(char::is_whitespace)
        .expect("Error parsing CARD/UNIQUE_ID");
    let mut config = DeviceConfig::default();
    match id_type {
        "CARD:" => config.card = Some(id.trim().parse().expect("Invalid CARD #")),
        "UNIQUE_ID:" => config.device_id = Some(u64::from_str_radix(id.trim(), 16).expect("Invalid UNIQUE_ID #")),
        _ => panic!("Unknown target device: Check /sys/class/drm"),
    }
    validate_detect_mount_points(&mut config);

    if config.card.is_some() {
        println!("---------- Card {} Settings ----------", config.card.unwrap());
    } else if config.device_id.is_some() {
        // TODO: Use pci-ids to get device name (Need to wait for pci-ids for subvendor entries)
        println!("---------- Device {:x} Settings ----------", config.device_id.unwrap());
    }

    // PERFORMANCE_LEVEL
    let file = File::open(config.home_path.join("power_dpm_force_performance_level"))
        .expect("Can't access power_dpm_force_performance_level");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("PERFORMANCE_LEVEL: {}\n", line);
    }

    // POWER_PROFILE
    let file = File::open(config.home_path.join("pp_power_profile_mode"))
        .expect("Can't access pp_power_profile_mode");
    println!("POWER_PROFILE_INDEX");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.contains("*") {
            println!("{}\n", line);
        }
    }

    // POWER_CAP
    let file = File::open(config.hwmon_path.join("power1_cap"))
        .expect("Can't access power1_cap file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("POWER_CAP:\n{} ({} W)", line, line.parse::<f32>().unwrap() / 1e6);
    }
    println!();

    // PP_OD_CLK_VOLTAGE
    let file = File::open(config.home_path.join("pp_od_clk_voltage"))
        .expect("Can't access pp_od_clk_voltage file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("{}", line);
    }
    println!();

    // FAN SETTINGS
    let fan_dir = config.home_path.join("gpu_od/fan_ctrl");
    let fan_target_temp_file = File::open(fan_dir.join("fan_target_temperature"))
        .expect("Can't access fan_target_temperature file");
    for line in BufReader::new(fan_target_temp_file).lines().map_while(Result::ok) {
        println!("{}", line);
    }
    let fan_zero_rpm_file_result = File::open(fan_dir.join("fan_zero_rpm_enable"));
    if let Ok(fan_zero_rpm_file) = fan_zero_rpm_file_result {
        println!();
        for line in BufReader::new(fan_zero_rpm_file).lines().map_while(Result::ok) {
            println!("{}", line);
        }
    }
    let fan_zero_rpm_stop_temp_file_result = File::open(
        fan_dir.join("fan_zero_rpm_stop_temperature"));
    if let Ok(fan_zero_rpm_stop_temp_file) = fan_zero_rpm_stop_temp_file_result {
        println!();
        for line in BufReader::new(fan_zero_rpm_stop_temp_file).lines().map_while(Result::ok) {
            println!("{}", line);
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct CliArgs {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List device settings
    Info {
        /// Device profile (card num in the profile) to read GPU info from
        #[arg(default_value_t=String::from("default"))]
        profile: String,
    },
    /// Set a device profile
    Set {
        /// Device profile
        #[arg(default_value_t=String::from("default"))]
        profile: String,
    },
    /// Reset a device
    Reset {
        /// Device profile (card num in the profile) to reset
        #[arg(default_value_t=String::from("default"))]
        profile: String,
    },
}

fn main() {
    let args  = CliArgs::parse();

    match args.command {
        Some(Commands::Set{profile}) => {
            let config_profile = CONFIG_PROFILE_PATH.to_owned() + &profile;
            let config = parse_profile(&config_profile);
            reset_settings(&config_profile);
            apply_settings(&profile, config);
        },
        Some(Commands::Reset{profile}) => {
            let config_profile = CONFIG_PROFILE_PATH.to_owned() + &profile;
            reset_settings(&config_profile);
        },
        Some(Commands::Info{profile}) => {
            let config_profile = CONFIG_PROFILE_PATH.to_owned() + &profile;
            read_card_settings(&config_profile);
        },
        None => {
            let config_profile = CONFIG_PROFILE_PATH.to_owned() + "default";
            read_card_settings(&config_profile);
        }
    };
}
