use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use clap::{Parser, Subcommand};

const CONFIG_PROFILE_PATH: &str = "/etc/default/amdgpu-settings.";
const DEFAULT_CARD_NUM: u8 = 1;

#[derive(Default, Debug)]
struct DeviceConfig {
    card: u32,
    od_sclk_min: Option<u32>,
    od_sclk_max: Option<u32>,
    od_mclk_min: Option<u32>,
    od_mclk_max: Option<u32>,
    od_vddgfx_offset: Option<i32>,
    power_cap: Option<u64>,
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

fn parse_od_mclk(_config: &mut DeviceConfig, _lines: &[String]) {
    println!("Skipping: custom OD_MCLK not implemented");
}

fn parse_od_vddgfx_offset(config: &mut DeviceConfig, lines: &[String]) {
    let value: i32 = lines[1].split("m")
        .collect::<Vec<&str>>()[0]
        .parse().expect("Invalid voltage");
    config.od_vddgfx_offset = Some(value);
}

fn parse_power_cap(config: &mut DeviceConfig, lines: &[String]) {
    let value: u64 = lines[1].parse().expect("Invalid POWER_CAP");
    config.power_cap = Some(value);

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

    // println!("{:#?}", lines);

    let mut config = DeviceConfig{card: card.1, ..Default::default()};
    let mut i: usize = 0;
    while i < lines.len() {
        let line: &str = &lines[i].trim();
        match line {
            "OD_SCLK:" => parse_od_sclk(&mut config, &lines[i..]),
            "OD_MCLK:" => parse_od_mclk(&mut config, &lines[i..]),
            "OD_VDDGFX_OFFSET:" => parse_od_vddgfx_offset(&mut config, &lines[i..]),
            "POWER_CAP:" => parse_power_cap(&mut config, &lines[i..]),
            _ => {}
        }
        i += 1;
    }
    // println!("{:#?}", config);
    return config;
}

fn apply_settings(name: &str, config: &DeviceConfig) {
    let home_path = format!("/sys/class/drm/card{}/device", config.card);
    println!("---------- {} Settings ----------", name.to_uppercase());
    println!("{:#?}", config);
    if config.power_cap.is_some() {
        let mut file = OpenOptions::new().write(true)
            .open(format!("{}/hwmon/hwmon1/power1_cap", home_path))
            .expect("Can't access power1_cap file");
        write!(&mut file, "{}", config.power_cap.unwrap())
            .expect("Failed to set POWER_CAP");
    }
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    if config.od_sclk_min.is_some() {
        write!(&mut file, "{}", format!("s 0 {}", config.od_sclk_min.unwrap()))
            .expect("Failed to set od_sclk_min");
    }
    if config.od_sclk_max.is_some() {
        write!(&mut file, "{}", format!("s 1 {}", config.od_sclk_max.unwrap()))
            .expect("Failed to set od_sclk_max");
    }
    if config.od_vddgfx_offset.is_some() {
        write!(&mut file, "{}", format!("vo {}", config.od_vddgfx_offset.unwrap()))
            .expect("Failed to set od_vddgfx_offset");
    }
    write!(&mut file, "c")
        .expect("Failed to final commit pp_od_clk_voltage_file");
    // println!("--------------------------------------");
    println!("Success!");
}

fn reset_settings(card_num: u8) {
    let home_path = format!("/sys/class/drm/card{}/device", card_num);
    let mut file = OpenOptions::new().write(true)
        .open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    println!("Resetting card{}...", card_num);
    write!(&mut file, "r")
        .expect("Failed to reset card with pp_od_clk_voltage_file");
    println!("Success!");
}

fn list_settings(card_num: u8) {
    println!("---------- Card {} Settings ----------", card_num);
    let home_path = format!("/sys/class/drm/card{}/device", card_num);
    let file = File::open(format!("{}/hwmon/hwmon1/power1_cap", home_path))
        .expect("Can't access power1_cap file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("POWER_CAP: {} ({} W)", line, line.parse::<f32>().unwrap() / 1e6);
    }
    let file = File::open(format!("{}/pp_od_clk_voltage", home_path))
        .expect("Can't access pp_od_clk_voltage file");
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        println!("{}", line);
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
    /// List current device config
    List {
        /// Device card number
        #[arg(default_value_t=1)]
        card: u8,
    },
    /// Set a device config profile
    Set {
        /// Device config profile
        #[arg(default_value_t=String::from("default"))]
        profile: String,
    },
    /// Reset device config profile
    Reset {
        /// Device card number
        #[arg(default_value_t=1)]
        card: u8,
    },
}

fn main() {
    let args  = CliArgs::parse();
    // println!("{:#?}", args);

    match args.command {
        Some(Commands::Set{profile}) => {
            let config_profile = CONFIG_PROFILE_PATH.to_owned() + &profile;
            let config = parse_profile(&config_profile);
            apply_settings(&profile, &config);
        },
        Some(Commands::Reset{card}) => {
            reset_settings(card);
        },
        Some(Commands::List{card}) => {
            list_settings(card);
        },
        None => {
            list_settings(DEFAULT_CARD_NUM);
        }
    };
}
