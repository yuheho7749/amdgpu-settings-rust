use std::fs::File;
use std::io::{prelude::*, BufRead, BufReader};
use std::path::Path;

const DEFAULT_CONFIG_PROFILE: &str = "/etc/default/amdgpu-settings.config0";

#[derive(Default)]
#[derive(Debug)]
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
            (&lines[i][3..].split("m").collect::<Vec<&str>>()[0])
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

fn parse_configs(path: &str) -> Option<DeviceConfig> {
    let file = File::open(path).expect("No config file");
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
    println!("{:#?}", config);
    return Some(config);
}

fn apply_settings(config: DeviceConfig) {

}

fn main() {
    // TODO: Read command line arg to get custom card profile
    // TEMP: Use the DEFAULT_CONFIG_PROFILE
    let config = parse_configs(DEFAULT_CONFIG_PROFILE)
        .expect("Error parsing config file!");
    apply_settings(config);
}
