use std::fs::{File, OpenOptions};
use std::path::Path;
use std::io::{BufRead, BufReader, Write};
use clap::{Parser, Subcommand};
use glob::glob;

const CONFIG_PROFILE_PATH: &str = "/etc/default/amdgpu-settings.";

#[derive(Default, Debug)]
struct DeviceConfig {
    card: u8,
    home_path: String,
    hwmon_path: String,
    performance_level: Option<String>,
    power_profile_index: Option<u8>,
    od_sclk_min: Option<u32>, // RNDA 3 or older
    od_sclk_max: Option<u32>, // RNDA 3 or older
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
    let home_path = format!("/sys/class/drm/card{}", config.card);
    if !Path::new(&home_path).exists() {
        panic!("Fatal error: Unable to locate card mount point. Please check /sys/class/drm/card# and update gpu profile.");
    }
    config.home_path = format!("{}/device", home_path);
    for entry in glob(&format!("{}/hwmon/hwmon*", config.home_path)).expect("Failed to detect hwmon path") {
        if let Ok(path) = entry {
            if let Some(path_str) = path.to_str() {
                config.hwmon_path = path_str.to_string();
                return;
            }
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
    let mut i: usize = 1;

    while lines[i].len() != 0 {
        let sclk: (char, i32) = (
            lines[i].chars().nth(0).expect("Invalid OD_SCLK_OFFSET option"),
            (&lines[i][3..].split("M").collect::<Vec<&str>>()[0])
                .parse().expect("Invalid OD_SCLK_OFFSET option")
        );
        match sclk.0 {
            '1' => config.od_sclk_offset = Some(sclk.1),
            _ => {
                println!("Invalid OD_SCLK_OFFSET option");
                return;
            },
        }
        i += 1;
    }
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
    let card: &str = &lines[0];
    let card: (&str, u8) = (&card[..4], (&card[6..])
        .parse().expect("Invalid card mount point: Check /sys/class/drm/card#"));

    let mut config = DeviceConfig{card: card.1, ..Default::default()};
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
    // let home_path = format!("/sys/class/drm/card{}/device", config.card);
    println!("---------- {} Settings ----------", name.to_uppercase());
    println!("{:#?}", config);

    // PERFORMANCE_LEVEL
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/power_dpm_force_performance_level", config.home_path))
        .expect("Can't access power_dpm_force_performance_level file");
    if config.performance_level.is_some() {
        write!(&mut file, "{}", config.performance_level.as_deref().unwrap_or("manual"))
            .expect("Failed to write power_dpm_force_performance_level");
    } else {
        write!(&mut file, "manual")
            .expect("Failed to write power_dpm_force_performance_level to \"manual\"");
    }

    // POWER_CAP (Side effect of writing a new value will reset GPU settings. Should set this
    // before adjusting the other settings)
    if config.power_cap.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/power1_cap", config.hwmon_path))
            .expect("Can't access power1_cap file");
        write!(&mut file, "{}", config.power_cap.unwrap())
            .expect("Failed to set POWER_CAP");
    }

    // POWER_PROFILE_INDEX
    if config.power_profile_index.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/pp_power_profile_mode", config.home_path))
            .expect("Can't access pp_power_profile_mode file");
        write!(&mut file, "{}", config.power_profile_index.unwrap())
            .expect("Failed to set POWER_PROFILE_INDEX");
    }

    // FAN_TARGET_TEMPERATURE 
    if config.fan_target_temp.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/gpu_od/fan_ctrl/fan_target_temperature", config.home_path))
            .expect("Can't access fan_target_temperature file");
        write!(&mut file, "{}", format!("{}\n", config.fan_target_temp.unwrap()))
            .expect("Failed to write FAN_TARGET_TEMPERATURE");
    }
    // FAN_ZERO_RPM_ENABLE
    if config.fan_zero_rpm.is_some() {
        let file_result = OpenOptions::new().write(true)
            .open(format!("{}/gpu_od/fan_ctrl/fan_zero_rpm_enable", config.home_path));
        if let Ok(mut file) = file_result {
            write!(&mut file, "{}", format!("{}\n", config.fan_zero_rpm.unwrap()))
                .expect("Failed to write FAN_ZERO_RPM_ENABLE");
        } else {
            println!("Skip setting FAN_ZERO_RPM_ENABLE. Make sure to have Linux 6.13 or newer.");
        }
    }
    // FAN_ZERO_RPM_STOP_TEMPERATURE
    if config.fan_zero_rpm_stop_temp.is_some() {
        let file_result = OpenOptions::new().write(true)
            .open(format!("{}/gpu_od/fan_ctrl/fan_zero_rpm_stop_temperature", config.home_path));
        if let Ok(mut file) = file_result {
            write!(&mut file, "{}", format!("{}\n", config.fan_zero_rpm_stop_temp.unwrap()))
                .expect("Failed to write FAN_ZERO_RPM_STOP_TEMPERATURE");
        } else {
            println!("Skip setting FAN_ZERO_RPM_STOP_TEMPERATURE. Make sure to have Linux 6.13 or newer.");
        }
    }

    // pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", config.home_path))
        .expect("Can't access pp_od_clk_voltage file");

    // OD_SCLK_OFFSET (RDNA 4)
    if config.od_sclk_offset.is_some() {
        if config.od_sclk_offset.is_some() {
            write!(&mut file, "{}", format!("s {}", config.od_sclk_offset.unwrap()))
                .expect("Failed to write od_sclk_offset");
            // file.write_all(format!("s {}", config.od_sclk_offset.unwrap()).as_bytes())
            //     .expect("Failed to write od_sclk_offset");
        }
    } else { // OD_SCLK (RDNA 3 or older)
        if config.od_sclk_min.is_some() {
            write!(&mut file, "{}", format!("s 0 {}", config.od_sclk_min.unwrap()))
                .expect("Failed to write od_sclk_min");
        }
        if config.od_sclk_max.is_some() {
            write!(&mut file, "{}", format!("s 1 {}", config.od_sclk_max.unwrap()))
                .expect("Failed to write od_sclk_max");
        }
    }
    // OD_MCLK
    if config.od_mclk_min.is_some() {
        write!(&mut file, "{}", format!("m 0 {}", config.od_mclk_min.unwrap()))
            .expect("Failed to write od_mclk_min");
    }
    if config.od_mclk_max.is_some() {
        write!(&mut file, "{}", format!("m 1 {}", config.od_mclk_max.unwrap()))
            .expect("Failed to write od_mclk_max");
    }
    // OD_VDDGFX_OFFSET
    if config.od_vddgfx_offset.is_some() {
        write!(&mut file, "{}", format!("vo {}", config.od_vddgfx_offset.unwrap()))
            .expect("Failed to write od_vddgfx_offset");
    }
    // NOTE: Commit to pp_od_clk_voltage (but it will actually just commit all "committable" settings on RDNA 3)
    // By "committable", see https://docs.kernel.org/gpu/amdgpu/thermal.html for all settings that require an explicit "c" to commit
    write!(&mut file, "c")
        .expect("Failed to commit final settings");
    println!("Success!");
}

fn reset_settings(path: &str) {
    let file = File::open(path).expect("Profile not found");
    let lines: Vec<String> = BufReader::new(file)
        .lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let card: &str = &lines[0];
    let card_num: u8 = (&card[6..]).parse().expect("Invalid card mount point: Check /sys/class/drm/card#");

    let mut config = DeviceConfig{card: card_num, ..Default::default()};
    validate_detect_mount_points(&mut config);
    println!("Resetting card{}...", config.card);
    // let home_path = format!("/sys/class/drm/card{}/device", card_num);

    // Reset PERFORMANCE_LEVEL
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/power_dpm_force_performance_level", config.home_path))
        .expect("Can't access power_dpm_force_performance_level file");
    write!(&mut file, "auto")
        .expect("Failed to reset power_dpm_force_performance_level to \"auto\"");

    // Reset POWER_CAP
    // TODO: fix hardcode hwmon1
    let file = File::open(format!("{}/power1_cap_default", config.hwmon_path))
        .expect("Can't access power1_cap file");
    let power_cap_default_lines: Vec<String> = BufReader::new(file).lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let power_cap_default = &power_cap_default_lines[0];
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/power1_cap", config.hwmon_path))
        .expect("Can't access power1_cap file");
    write!(&mut file, "{}", power_cap_default)
        .expect("Failed to reset POWER_CAP");

    // Reset POWER_PROFILE_INDEX
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_power_profile_mode", config.home_path))
        .expect("Can't access pp_od_clk_voltage file");
    write!(&mut file, "0") // 0 is BOOTUP_DEFAULT
        .expect("Failed to reset power profile to BOOTUP_DEFAULT");

    // Reset pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", config.home_path))
        .expect("Can't access pp_od_clk_voltage file");
    write!(&mut file, "r")
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
    let card: &str = &lines[0];
    let card_num: u8 = (&card[6..]).parse().expect("Invalid card mount point: Check /sys/class/drm/card#");

    let mut config = DeviceConfig{card: card_num, ..Default::default()};
    validate_detect_mount_points(&mut config);

    println!("---------- Card {} Settings ----------", config.card);
    // let home_path = format!("/sys/class/drm/card{}/device", card_num);
    // PERFORMANCE_LEVEL
    let file = File::open(format!("{}/power_dpm_force_performance_level", config.home_path))
        .expect("Can't access power_dpm_force_performance_level");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("PERFORMANCE_LEVEL: {}\n", line);
    }

    // POWER_PROFILE
    let file = File::open(format!("{}/pp_power_profile_mode", config.home_path))
        .expect("Can't access pp_power_profile_mode");
    println!("POWER_PROFILE_INDEX");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.contains("*") {
            println!("{}\n", line);
        }
    }

    // POWER_CAP
    let file = File::open(format!("{}/power1_cap", config.hwmon_path))
        .expect("Can't access power1_cap file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("POWER_CAP:\n{} ({} W)", line, line.parse::<f32>().unwrap() / 1e6);
    }
    println!();
    // PP_OD_CLK_VOLTAGE
    let file = File::open(format!("{}/pp_od_clk_voltage", config.home_path))
        .expect("Can't access pp_od_clk_voltage file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("{}", line);
    }
    // FAN SETTINGS
    let fan_dir = format!("{}/gpu_od/fan_ctrl", config.home_path);
    let fan_target_temp_file = File::open(format!("{}/fan_target_temperature", fan_dir))
        .expect("Can't access fan_target_temperature file");
    for line in BufReader::new(fan_target_temp_file).lines().map_while(Result::ok) {
        println!("{}", line);
    }
    let fan_zero_rpm_file_result = File::open(format!("{}/fan_zero_rpm_enable", fan_dir));
    if let Ok(fan_zero_rpm_file) = fan_zero_rpm_file_result {
        println!();
        for line in BufReader::new(fan_zero_rpm_file).lines().map_while(Result::ok) {
            println!("{}", line);
        }
    }
    let fan_zero_rpm_stop_temp_file_result = File::open(format!("{}/fan_zero_rpm_stop_temperature", fan_dir));
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
