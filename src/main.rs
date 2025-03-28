use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use clap::{Parser, Subcommand};

const CONFIG_PROFILE_PATH: &str = "/etc/default/amdgpu-settings.";

#[derive(Default, Debug)]
struct DeviceConfig {
    card: u32,
    od_sclk_min: Option<u32>,
    od_sclk_max: Option<u32>,
    od_mclk_min: Option<u32>,
    od_mclk_max: Option<u32>,
    od_vddgfx_offset: Option<i32>,
    power_cap: Option<u64>,
    fan_target_temp: Option<u32>,
    fan_zero_rpm: Option<u8>,
    fan_zero_rpm_stop_temp: Option<u32>,
}

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
    let card: (&str, u32) = (&card[..4], (&card[6..])
        .parse().expect("Invalid card mount point: Check /sys/class/drm/card#"));

    let mut config = DeviceConfig{card: card.1, ..Default::default()};
    let mut i: usize = 0;
    while i < lines.len() {
        let line: &str = &lines[i].trim();
        match line {
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

fn apply_settings(name: &str, config: &DeviceConfig) {
    let home_path = format!("/sys/class/drm/card{}/device", config.card);
    println!("---------- {} Settings ----------", name.to_uppercase());
    println!("{:#?}", config);
    // POWER_CAP (Side effect of writing a new value will reset GPU settings. Should set this
    // before adjusting the other settings)
    if config.power_cap.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/hwmon/hwmon1/power1_cap", home_path))
            .expect("Can't access power1_cap file");
        write!(&mut file, "{}", config.power_cap.unwrap())
            .expect("Failed to set POWER_CAP");
    }

    // FAN_TARGET_TEMPERATURE 
    if config.fan_target_temp.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/gpu_od/fan_ctrl/fan_target_temperature", home_path))
            .expect("Can't access fan_target_temperature file");
        write!(&mut file, "{}", format!("{}\n", config.fan_target_temp.unwrap()))
            .expect("Failed to write FAN_TARGET_TEMPERATURE");
    }
    // FAN_ZERO_RPM_ENABLE
    if config.fan_zero_rpm.is_some() {
        let file_result = OpenOptions::new().write(true)
            .open(format!("{}/gpu_od/fan_ctrl/fan_zero_rpm_enable", home_path));
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
            .open(format!("{}/gpu_od/fan_ctrl/fan_zero_rpm_stop_temperature", home_path));
        if let Ok(mut file) = file_result {
            write!(&mut file, "{}", format!("{}\n", config.fan_zero_rpm_stop_temp.unwrap()))
                .expect("Failed to write FAN_ZERO_RPM_STOP_TEMPERATURE");
        } else {
            println!("Skip setting FAN_ZERO_RPM_STOP_TEMPERATURE. Make sure to have Linux 6.13 or newer.");
        }
    }

    // pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    // OD_SCLK
    if config.od_sclk_min.is_some() {
        write!(&mut file, "{}", format!("s 0 {}", config.od_sclk_min.unwrap()))
            .expect("Failed to write od_sclk_min");
    }
    if config.od_sclk_max.is_some() {
        write!(&mut file, "{}", format!("s 1 {}", config.od_sclk_max.unwrap()))
            .expect("Failed to write od_sclk_max");
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
    // NOTE: Commit to pp_od_clk_voltage (but it will actually just commit all "committable" settings)
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

    let home_path = format!("/sys/class/drm/card{}/device", card_num);

    // Reset POWER_CAP
    let file = File::open(format!("{}/hwmon/hwmon1/power1_cap_default", home_path))
        .expect("Can't access power1_cap file");
    let power_cap_default_lines: Vec<String> = BufReader::new(file).lines()
        .map(|l| l.expect("Can't parse line"))
        .collect();
    let power_cap_default = &power_cap_default_lines[0];
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/hwmon/hwmon1/power1_cap", home_path))
        .expect("Can't access power1_cap file");
    write!(&mut file, "{}", power_cap_default)
        .expect("Failed to reset POWER_CAP");

    // Reset pp_od_clk_voltage
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    println!("Resetting card{}...", card_num);
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

    println!("---------- Card {} Settings ----------", card_num);
    // POWER_CAP
    let home_path = format!("/sys/class/drm/card{}/device", card_num);
    let file = File::open(format!("{}/hwmon/hwmon1/power1_cap", home_path))
        .expect("Can't access power1_cap file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("POWER_CAP: {} ({} W)", line, line.parse::<f32>().unwrap() / 1e6);
    }
    println!();
    // PP_OD_CLK_VOLTAGE
    let file = File::open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("{}", line);
    }
    // FAN SETTINGS
    let fan_dir = format!("{}/gpu_od/fan_ctrl", home_path);
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
            apply_settings(&profile, &config);
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
