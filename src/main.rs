extern crate alloc;
extern crate async_msp_lib;
extern crate multiwii_serial_protocol_v2;

extern crate serialport;

use serialport::{available_ports, open_with_settings};

extern crate packed_struct;
extern crate packed_struct_codegen;

use async_std::fs::OpenOptions;
use async_std::io::{BufReader, BufWriter};
use async_std::io::prelude::*;

use async_std::prelude::*;

use std::time::Duration;
use std::iter::Iterator;
use std::str::FromStr;
use futures::future::try_join_all;
use futures::stream::FuturesUnordered;
use clap_v3::{App, AppSettings, Arg};
use multiwii_serial_protocol_v2::structs::*;
use std::convert::TryInto;
use std::collections::HashMap;
use packed_struct::PrimitiveEnum;
use async_msp_lib::{INavMsp, SettingInfo};
use itertools::Itertools;


static FEATURE_NAMES: [&str; 32] = [
    "THR_VBAT_COMP", "VBAT", "TX_PROF_SEL", "BAT_PROF_AUTOSWITCH", "MOTOR_STOP",
    "", "SOFTSERIAL", "GPS", "",
    "", "TELEMETRY", "CURRENT_METER", "3D", "RX_PARALLEL_PWM",
    "RX_MSP", "RSSI_ADC", "LED_STRIP", "DASHBOARD", "",
    "BLACKBOX", "", "TRANSPONDER", "AIRMODE",
    "SUPEREXPO", "VTX", "RX_SPI", "", "PWM_SERVO_DRIVER", "PWM_OUTPUT_ENABLE",
    "OSD", "FW_LAUNCH", "",
];

#[async_std::main]
async fn main() {

    // TODO: implement port arguments

    let matches = App::new("msp")
        .version("0.0.1")
        .author("Ilya G. <amfernusus@gmail.com>")
        .about("Interact with msp flight controller")
        .subcommand(
            App::new("setting")
                .about("Common setting")
                .subcommand(
                    App::new("list").about("Get all common setting")
                )
                .subcommand(
                    App::new("set")
                        .about("Set common setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("name").help("The setting name to set").required(true).takes_value(true))
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
                .subcommand(
                    App::new("get")
                        .about("get common setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("name").help("The setting name to set").required(true).takes_value(true))
                )
                .subcommand(
                    App::new("set-all")
                        .about("Set all common setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("input").help("settings file path").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("aux")
                .about("Get all aux setting")
                .subcommand(
                    App::new("set")
                        .about("Set aux setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("mmix")
                .about("Get all mmix setting")
                .subcommand(
                    App::new("set")
                        .about("Set mmix setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("smix")
                .about("Get all smix setting")
                .subcommand(
                    App::new("set")
                        .about("Set smix setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("servo")
                .about("Get all servo setting")
                .subcommand(
                    App::new("set")
                        .about("Set servo setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("map")
                .about("Get all rx map setting")
                .subcommand(
                    App::new("set")
                        .about("Set rx map setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("serial")
                .about("Get all serial setting")
                .subcommand(
                    App::new("set")
                        .about("Set serial setting")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("osd_item")
                .about("Get all osd items (derecated)")
                .subcommand(
                    App::new("set")
                        .about("Set osd item")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("osd_layout")
                .about("Get all osd_layout items")
                .subcommand(
                    App::new("set")
                        .about("Set osd layout item")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                        .arg(
                            Arg::with_name("strict")
                                .long("strict")
                                .help("stop if setting not found in fc")
                                .required(false)
                        )
                )
        )
        .subcommand(
            App::new("feature")
                .about("Get all features")
                .subcommand(
                    App::new("set")
                        .about("Set features(FEATURE_NAME:enable, -FEATURE_NAME:disable)")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
                .subcommand(
                    App::new("enable")
                        .about("Enable feature")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
                .subcommand(
                    App::new("disable")
                        .about("Disable feature")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("value").help("The setting value to set").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("blackbox")
                .about("Onboard Blackbox")
                .subcommand(
                    App::new("download")
                        .about("Download blackbox concurrently")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("input").help("download path").required(true).takes_value(true))
                )
                .subcommand(
                    App::new("downloadv2")
                        .about("Pull blackbox serially")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("input").help("download path").required(true).takes_value(true))
                )
        )
        .subcommand(
            App::new("config")
                .about("Get all configs")
                .subcommand(
                    App::new("set")
                        .about("Upload all configs")
                        .setting(AppSettings::ArgRequiredElseHelp)
                        .arg(Arg::with_name("input").help("settings file path").required(true).takes_value(true))
                        .arg(
                            Arg::with_name("strict")
                                .long("strict")
                                .help("stop if setting not found in fc")
                                .required(false)
                        )
                )
        )
        .subcommand(
            App::new("reboot")
                .about("Reboot the device")
        )
        .arg(
            Arg::with_name("port")
                .short('p')
                .long("port")
                .help("device serial port")
                .required(false)
                .takes_value(true)
        )
        .arg(
            Arg::with_name("save")
                .short('s')
                .long("save")
                .help("settings file path")
                .required(false)
        )
        .arg(
            Arg::with_name("reboot")
                .short('r')
                .long("reboot")
                .help("reboot fc")
                .required(false)
        )
        .get_matches();


    let s = serialport::SerialPortSettings {
        baud_rate: 115200,
        data_bits: serialport::DataBits::Eight,
        flow_control: serialport::FlowControl::None,
        parity: serialport::Parity::None,
        stop_bits: serialport::StopBits::One,
        timeout: Duration::from_millis(1),
    };

    let port = match matches.value_of("port") {
        Some(p) => String::from(p),
        None =>  available_ports()
            .expect("No serial ports")
            .first()
            .expect("No serial ports, specify with -p")
            .port_name
            .clone()
    };

    // TODO: what stop and start bits are inav using, is every one just using the canonical defalts?
    let serialport = open_with_settings(&port, &s)
        .expect("Failed to open serial port");

    // green-thread 1: read into input channel from serial(reading from serial is blocking)
    let inav = INavMsp::new();
    inav.start(serialport);

    match matches.subcommand() {
        ("setting", Some(setting_matches)) => {
            match setting_matches.subcommand() {
                ("list", Some(_)) => {
                    let dump = dump_common_setting(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                }
                ("get", Some(get_matches)) => {
                    if !get_matches.is_present("name") {
                        unreachable!();
                    }

                    let name = get_matches.value_of("name").unwrap();
                    let setting_info = inav.get_setting_info_by_name(&name).await.unwrap();
                    println!("{}", setting_to_str(&setting_info));
                }
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("name") || !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let name = set_matches.value_of("name").unwrap();
                    let value = set_matches.value_of("value").unwrap();
                    let setting_info = inav.get_setting_info_by_name(&name).await.unwrap();
                    let payload = setting_to_vec(&setting_info, value).unwrap();
                    inav.set_setting_by_name(name, &payload).await.unwrap();
                },
                ("set-all", Some(set_all_matches)) => {
                    if !set_all_matches.is_present("input") {
                        print!("missing input");
                    }

                    let input = set_all_matches.value_of("input").unwrap();
                    let f = OpenOptions::new()
                        .read(true)
                        .open(input)
                        .await.unwrap();
                    let f = BufReader::new(f);

                    let settings_list = f.lines().map(|l| l.unwrap()).collect().await;

                    upload_common_settings(&inav, settings_list, true).await.unwrap();
                },
                ("", None) => println!("No subcommand was used"),
                _ => unreachable!(),
            }
        }
        ("aux", Some(aux_matches)) => {
            match aux_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_aux(&inav, value).await.unwrap();
                },
                ("", None) => {
                    let dump = dump_aux(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("mmix", Some(mmix_matches)) => {
            match mmix_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_mmix(&inav, value).await.unwrap();
                },
                ("", None) => {
                    let dump = dump_mmix(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("smix", Some(smix_matches)) => {
            match smix_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_smix(&inav, value).await.unwrap();
                },
                ("", None) => {
                    let dump = dump_smix(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("servo", Some(servo_matches)) => {
            match servo_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_servo(&inav, value).await.unwrap();
                },
                ("", None) => {
                    let dump = dump_servo(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("map", Some(rx_map_matches)) => {
            match rx_map_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();

                    upload_map(&inav, value).await.unwrap();
                },
                ("", None) => {
                    println!("map {}", dump_map(&inav).await.unwrap());
                },
                _ => unreachable!(),
            }
        }
        ("serial", Some(serial_matches)) => {
            match serial_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_serial(&inav, value).await.unwrap();
                },
                ("", None) => {
                    let dump = dump_serial(&inav).await.unwrap();
                    for d in dump {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("osd_item", Some(osd_item_matches)) => {
            match osd_item_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = set_matches.value_of("value").unwrap();
                    upload_osd_item(&inav, value).await.unwrap();
                },
                ("", None) => {
                    for d in dump_osd_items(&inav).await.unwrap() {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("osd_layout", Some(osd_layout_matches)) => {
            match osd_layout_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    let is_strict = set_matches.is_present("strict");

                    let value = set_matches.value_of("value").unwrap();
                    upload_osd_layout_items(&inav, vec![value.to_string()], is_strict).await.unwrap();
                },
                ("", None) => {
                    for d in dump_osd_layouts(&inav).await.unwrap() {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("feature", Some(serial_matches)) => {
            match serial_matches.subcommand() {
                ("set", Some(set_matches)) => {
                    if !set_matches.is_present("value") {
                        unreachable!();
                    }

                    // because of the async nature of msp-lib, enable and disable calls must be chained
                    let value = set_matches.value_of("value").unwrap();
                    let values = value.split_whitespace().collect();
                    upload_features(&inav, values).await.unwrap();
                },
                ("enable", Some(enable_matches)) => {
                    if !enable_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = enable_matches.value_of("value").unwrap();
                    enable_feature(&inav, value).await.unwrap();
                },
                ("disable", Some(disable_matches)) => {
                    if !disable_matches.is_present("value") {
                        unreachable!();
                    }

                    let value = disable_matches.value_of("value").unwrap();
                    disable_feature(&inav, value).await.unwrap();
                }
                ("", None) => {
                    for d in dump_feature(&inav).await.unwrap() {
                        println!("{}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("blackbox", Some(blackbox_matches)) => {
            match blackbox_matches.subcommand() {
                ("download", Some(download_matches)) => {
                    if !download_matches.is_present("input") {
                        print!("missing input");
                    }
                    let input = download_matches.value_of("input").unwrap();

                    let mut f = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(input)
                        .await.unwrap();

                    // no point going more then this size because inav won't return bigger chunk
                    let blackbox_data = inav.read_flash_data(0x1000, |chunk, total| {
                        println!("received packet: {:?}/{:?}", chunk, total);
                    }).await.unwrap();

                    f.write(&blackbox_data[..]).await.unwrap();
                    f.flush().await.unwrap();
                }
                ("downloadv2", Some(downloadv2_matches)) => {
                    if !downloadv2_matches.is_present("input") {
                        print!("missing input");
                    }

                    let input = downloadv2_matches.value_of("input").unwrap();
                    let mut flash_data_file = inav.open_flash_data().await;

                    let f = OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(input)
                        .await.unwrap();

                    let mut buf_writer = BufWriter::new(f);
                    loop {
                        let chunk = flash_data_file.read_chunk().await.unwrap();
                        if chunk.len() == 0 {
                            break;
                        }
                        buf_writer.write(&chunk[..]).await.unwrap();
                    }

                    buf_writer.flush().await.unwrap();
                },
                ("", None) => println!("No subcommand was used"),
                _ => unreachable!(),
            }
        }
        ("config", Some(config_matches)) => {
            match config_matches.subcommand() {
                ("set", Some(upload_matches)) => {
                    if !upload_matches.is_present("input") {
                        print!("missing input");
                    }

                    let is_strict = upload_matches.is_present("strict");

                    let input = upload_matches.value_of("input").unwrap();
                    let f = OpenOptions::new()
                        .read(true)
                        .open(input)
                        .await.unwrap();
                    let f = BufReader::new(f);

                    let valid_set_lines = f
                        .lines()
                        .map(|l| l.unwrap().to_string())
                        .map(|l| {
                            let parts = l
                                .splitn(2, '#')
                                .collect::<Vec<&str>>();

                            if parts.len() < 2 {
                                return l.to_string();
                            }

                            return parts[0].trim().to_string();
                        })
                        .filter(|l| l.len() > 0)
                        .fold(vec![], |mut acc, l| {
                            let set_command: Vec<String> = l
                                .splitn(2, ' ')
                                .map(|vals| vals.to_owned())
                                .collect();

                            if set_command.len() < 2 {
                                return acc;
                            }

                            acc.push((set_command[0].to_owned(), set_command[1].to_owned()));

                            return acc;
                        }).await;


                    let lookup = valid_set_lines.into_iter().into_group_map();

                    match lookup.get("mmix") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_mmix(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("mmix {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some mmix {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        }
                        None => (),
                    };

                    match lookup.get("smix") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_smix(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("smix {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some smix {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        },
                        None => (),
                    };

                    match lookup.get("servo") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_servo(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("servo {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some servo {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        },
                        None => (),
                    };

                    match lookup.get("serial") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_serial(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("serial {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some serial {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        },
                        None => (),
                    };

                    match lookup.get("aux") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_aux(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("aux {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some aux {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        },
                        None => (),
                    };

                    match lookup.get("map") {
                        Some(values) => {
                            let mut futures = values
                                .iter()
                                .map(|v| upload_map(&inav, v))
                                .collect::<FuturesUnordered<_>>();

                            loop {
                                match futures.next().await {
                                    Some(Ok(result)) => println!("map {}", result),
                                    Some(Err(e)) => {
                                        eprintln!("failed to set some map {}", e);
                                        if is_strict {
                                            return;
                                        }
                                    },
                                    None => break,
                                }
                            }
                        },
                        None => (),
                    };

                    match lookup.get("osd_layout") {
                        Some(values) => {
                            match upload_osd_layout_items(&inav, values.to_vec(), is_strict).await {
                                Ok(_) => (),
                                Err(_) => {
                                    if is_strict {
                                        return;
                                    }
                                },
                            };
                        },
                        None => (),
                    };

                    match lookup.get("feature") {
                        Some(values) => {
                            match upload_features(&inav, values.iter().map(|l| l.as_str()).collect()).await {
                                Ok(_) => println!("feature {}", values.join(" ")),
                                Err(e) => {
                                    eprintln!("failed to set features {}", e);
                                    if is_strict {
                                        return;
                                    }
                                },
                            }
                        },
                        None => (),
                    };

                    match lookup.get("set") {
                        Some(values) => upload_common_settings(&inav, values.to_vec(), is_strict).await.unwrap(),
                        None => (),
                    };

                    println!("Done!");

                },
                ("", None) => {
                    for d in dump_mmix(&inav).await.unwrap() {
                        println!("mmix {}", d);
                    }

                    for d in dump_smix(&inav).await.unwrap() {
                        println!("smix {}", d);
                    }

                    for d in dump_servo(&inav).await.unwrap() {
                        println!("servo {}", d);
                    }

                    for d in dump_serial(&inav).await.unwrap() {
                        println!("serial {}", d);
                    }

                    for d in dump_aux(&inav).await.unwrap() {
                        println!("aux {}", d);
                    }

                    println!("map {}", dump_map(&inav).await.unwrap());

                    for d in dump_osd_layouts(&inav).await.unwrap() {
                        println!("osd_layout {}", d);
                    }

                    for d in dump_feature(&inav).await.unwrap() {
                        println!("feature {}", d);
                    }

                    for d in dump_common_setting(&inav).await.unwrap() {
                        println!("set {}", d);
                    }
                },
                _ => unreachable!(),
            }
        }
        ("reboot", Some(_)) => {
            inav.reboot().await.unwrap();
        }
        ("", None) => println!("No subcommand was used"), // If no subcommand was usd it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }

    if matches.is_present("save") {
        inav.save_to_eeprom().await.unwrap();
    }

    if matches.is_present("reboot") {
        inav.reboot().await.unwrap();
    }
}

async fn describe_settings(inav: &INavMsp) -> Result<Vec<SettingInfo>, &str> {
    let pg_settings = inav.get_pg_settings().await.unwrap();
    let mut setting_ids: Vec<u16> = pg_settings
        .iter()
        .flat_map(|pg_s| (pg_s.start_id..=pg_s.end_id).map(u16::from).collect::<Vec<u16>>())
        .collect();

    setting_ids.sort();
    let setting_info_futures = setting_ids
        .iter()
        .map(|id| inav.get_setting_info_by_id(&id));
    return try_join_all(setting_info_futures).await;
}

async fn upload_aux<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();
    let index = split_iter.next().unwrap();
    let box_id = split_iter.next().unwrap();
    let aux_channel_index = split_iter.next().unwrap();
    let start_step = split_iter.next().unwrap();
    let end_step = split_iter.next().unwrap();

    let start_step_parsed = u32::from_str(start_step).unwrap();
    let end_step_parsed = u32::from_str(end_step).unwrap();

    let range = MspModeRange {
        box_id: u8::from_str(box_id).unwrap(),
        aux_channel_index: MspRcChannel::from_primitive(
            u8::from_str(aux_channel_index).unwrap()
        ).unwrap(),
        start_step: ((start_step_parsed - 900) / 25) as u8,
        end_step: ((end_step_parsed - 900) / 25) as u8,
    };

    inav.set_mode_range(u8::from_str(index).unwrap(), range).await?;
    Ok(value)
}

async fn dump_aux(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let ranges = inav.get_mode_ranges().await?;
    let dump: Vec<String> = ranges
        .iter()
        .enumerate()
        .map(|(i, r)| format!(
            "{} {} {} {} {}",
            i,
            r.box_id,
            r.aux_channel_index.to_primitive(),
            (r.start_step as u32) * 25 + 900,
            (r.end_step as u32) * 25 + 900)
        ).collect();

    return Ok(dump);
}

async fn upload_mmix<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();
    let index = split_iter.next().unwrap();
    let throttle = split_iter.next().unwrap();
    let roll = split_iter.next().unwrap();
    let pitch = split_iter.next().unwrap();
    let yaw = split_iter.next().unwrap();

    let mmix = MspMotorMixer {
        throttle: f32::from_str(throttle).unwrap() as u16 * 1000,
        roll: (f32::from_str(roll).unwrap() + 2f32) as u16 * 1000,
        pitch: (f32::from_str(pitch).unwrap() + 2f32) as u16 * 1000,
        yaw: (f32::from_str(yaw).unwrap() + 2f32) as u16 * 1000,
    };

    inav.set_motor_mixer(u8::from_str(index).unwrap(), mmix).await?;
    Ok(value)
}

async fn dump_mmix(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let mixers = inav.get_motor_mixers().await?;
    let dump: Vec<String> = mixers
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{} {:.3} {:.3} {:.3} {:.3}",
                              i,
                              m.throttle as f32 / 1000f32,
                              m.roll as f32 / 1000f32 - 2f32,
                              m.pitch as f32 / 1000f32 - 2f32,
                              m.yaw as f32 / 1000f32 - 2f32)
        ).collect();

    return Ok(dump);
}

async fn upload_smix<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();

    let index = split_iter.next().unwrap();
    let target_channel = split_iter.next().unwrap();
    let input_source = split_iter.next().unwrap();
    let rate = split_iter.next().unwrap();
    let speed = split_iter.next().unwrap();

    let smix = MspServoMixRule {
        target_channel: u8::from_str(target_channel).unwrap(),
        input_source: u8::from_str(input_source).unwrap(),
        rate: u16::from_str(rate).unwrap(),
        speed: u8::from_str(speed).unwrap(),
        min: 0,
        max: 100,
        box_id: 0,
    };

    inav.set_servo_mix_rule(u8::from_str(index).unwrap(), smix).await?;
    Ok(value)
}

async fn dump_servo(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let servo = inav.get_servo_configs().await?;
    let dump: Vec<String> = servo
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{} {} {} {} {}",
                              i,
                              s.min,
                              s.max,
                              s.middle,
                              s.rate)
        ).collect();

    return Ok(dump);
}

async fn upload_servo<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();

    // servo 0 1200 1950 1490 100
    let index = split_iter.next().unwrap();
    let min = split_iter.next().unwrap();
    let max = split_iter.next().unwrap();
    let middle = split_iter.next().unwrap();
    let rate = split_iter.next().unwrap();

    let servo = MspServoConfig {
        min: u16::from_str(min).unwrap(),
        max: u16::from_str(max).unwrap(),
        middle: u16::from_str(middle).unwrap(),
        rate: u8::from_str(rate).unwrap(),
        unused1: 0,
        unused2: 0,
        forward_from_channel: 255,
        reverse_input: 0, // Depracted, Input reversing is not required since it can be done on mixer level
    };

    inav.set_servo_config(u8::from_str(index).unwrap(), servo).await?;
    Ok(value)
}

async fn dump_smix(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let mixers = inav.get_servo_mix_rules().await?;
    let dump: Vec<String> = mixers
        .iter()
        .enumerate()
        .map(|(i, m)| format!("{} {} {} {} {} {}",
                              i,
                              m.target_channel,
                              m.input_source,
                              m.rate,
                              m.speed,
                              m.min)
        ).collect();

    return Ok(dump);
}

async fn upload_map<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let map_val = match value {
        "TAER" => [1, 2, 3, 0],
        "AETR" => [0, 1, 3, 2],
        _ => return Err("Unsupported map"),
    };

    let rx_map = MspRxMap {
        map: map_val,
    };

    inav.set_rx_map(rx_map).await?;
    Ok(value)
}

async fn dump_map(inav: &INavMsp) -> Result<String, &str> {
    let map = inav.get_rx_map().await?;

    let map_name = match map.map {
        [1, 2, 3, 0] => "TAER",
        [0, 1, 3, 2] => "AETR",
        _ => return Err("Unsupported map"),
    };

    Ok(map_name.to_owned())
}

async fn upload_serial<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();

    let identifier = split_iter.next().unwrap();
    let function_mask = split_iter.next().unwrap();
    let msp_baudrate_index = split_iter.next().unwrap();
    let gps_baudrate_index = split_iter.next().unwrap();
    let telemetry_baudrate_index = split_iter.next().unwrap();
    let peripheral_baudrate_index = split_iter.next().unwrap();

    let serial = MspSerialSetting {
        identifier: u8_to_serial_identifier(u8::from_str(identifier).unwrap()).unwrap(),
        function_mask: u32::from_str(function_mask).unwrap(),
        msp_baudrate_index: string_to_baudrate(msp_baudrate_index).unwrap(),
        gps_baudrate_index: string_to_baudrate(gps_baudrate_index).unwrap(),
        telemetry_baudrate_index: string_to_baudrate(telemetry_baudrate_index).unwrap(),
        peripheral_baudrate_index: string_to_baudrate(peripheral_baudrate_index).unwrap(),
    };

    inav.set_serial_settings(vec![serial]).await?;
    Ok(value)
}

async fn dump_serial(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let serials = inav.get_serial_settings().await?;
    let dump: Vec<String> = serials
        .iter()
        .map(|s| format!("{} {} {} {} {} {}",
                         s.identifier as u8,
                         s.function_mask,
                         baudrate_to_string(&s.msp_baudrate_index).unwrap(),
                         baudrate_to_string(&s.gps_baudrate_index).unwrap(),
                         baudrate_to_string(&s.telemetry_baudrate_index).unwrap(),
                         baudrate_to_string(&s.peripheral_baudrate_index).unwrap())
        ).collect();

    return Ok(dump);
}

async fn upload_osd_item<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let mut split_iter = value.split_whitespace();

    let item_pos = u8::from_str(split_iter.next().unwrap()).unwrap();
    let col = u8::from_str(split_iter.next().unwrap()).unwrap();
    let row = u8::from_str(split_iter.next().unwrap()).unwrap();
    let vis = split_iter.next().unwrap(); // v h
    let is_visible: u16 = match vis {
        "V" => 0x0800,
        "H" => 0,
        _ => 0
    };

    let field: u16 = ((col as u16) | ((row as u16) << 5)) | is_visible;
    let bytes = field.to_le_bytes();

    let item = MspOsdItemPosition {
        col: bytes[0],
        row: bytes[1],
    };

    inav.set_osd_config_item(item_pos, item).await?;
    Ok(value)
}

async fn dump_osd_items(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let osd = inav.get_osd_settings().await?;
    let dump: Vec<String> = osd.item_positions
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let field = u16::from_le_bytes([item.col, item.row]);

            format!(
                "{} {} {} {}",
                i,
                field & 0x001F, // OSD_X
                (field >> 5) & 0x001F, // OSD_Y
                if field & 0x0800 > 0 { "V" } else { "H" },
            )
        }).collect();

    return Ok(dump);
}

async fn upload_osd_layout_items<'a, 'b>(inav: &'a INavMsp, values: Vec<String>, strict: bool) -> Result<(), &'a str> {
    let layout_count = inav.get_osd_layout_count().await?;
    let items: Vec<(String, MspSetOsdLayoutItem)> = values.iter().map(|value| {
        let mut split_iter = value.split_whitespace();

        let layout_index = u8::from_str(split_iter.next().unwrap()).unwrap();
        let item_pos = u8::from_str(split_iter.next().unwrap()).unwrap();
        let col = u8::from_str(split_iter.next().unwrap()).unwrap();
        let row = u8::from_str(split_iter.next().unwrap()).unwrap();
        let vis = split_iter.next().unwrap(); // v h
        let is_visible: u16 = match vis {
            "V" => 0x0800,
            "H" => 0,
            _ => 0
        };

        let field: u16 = ((col as u16) | ((row as u16) << 5)) | is_visible;
        let bytes = field.to_le_bytes();

        (value.to_string(), MspSetOsdLayoutItem {
            layout_index: layout_index,
            item: MspSetOsdLayout {
                item_index: item_pos,
                item: MspOsdItemPosition {
                    col: bytes[0],
                    row: bytes[1],
                },
            },
        })
    }).collect();

    let items_res = items.iter().try_fold(vec![], |mut acc, (val, item)| {
        if &item.layout_index < &layout_count.layout_count && &item.item.item_index < &layout_count.item_count {
            acc.push((val, item));
        } else {
            eprintln!("unsupported osd_layout {} {}", &item.layout_index, &item.item.item_index);

            if strict {
                return None;
            }
        }

        return Some(acc)
    });

    let valid_items = match items_res {
        Some(buf) => buf,
        None => return Err("aborting due to unsupported settings")
    };

    let mut futures = valid_items
        .iter()
        .map(|(val, item)| _set_osd_layout_item(inav, val, item))
        .collect::<FuturesUnordered<_>>();

    loop {
        match futures.next().await {
            Some(Ok(_)) => (),
            Some(Err(e)) => {
                eprintln!("failed to set some osd_layout {}", e);
                if strict {
                    return Err("failed to set some item");
                }
            },
            None => break,
        }
    }

    Ok(())
}

async fn _set_osd_layout_item<'a>(inav: &'a INavMsp, val: &String, item: &MspSetOsdLayoutItem) -> Result<(), &'a str> {
    inav.set_osd_layout_item(item.layout_index, item.item).await?;
    println!("osd_layout {}", val);
    Ok(())
}

// iNav only command
async fn dump_osd_layouts(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let mut dump = vec![];

    let layouts = inav.get_osd_layouts().await?;

    for (layout_i, items) in layouts.iter().enumerate() {
        for (i, item) in items.iter().enumerate() {
            let field = u16::from_le_bytes([item.col, item.row]);
            let dump_item = format!(
                "{} {} {} {} {}",
                layout_i,
                i,
                field & 0x001F, // OSD_X
                (field >> 5) & 0x001F, // OSD_Y
                if field & 0x0800 > 0 { "V" } else { "H" },
            );

            dump.push(dump_item);
        }
    }

    return Ok(dump);
}

async fn enable_feature<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let index = FEATURE_NAMES.iter().position(|&n| n == value).unwrap();
    let mut feat = inav.get_features().await.unwrap();

    feat.features[index] = true;

    inav.set_features(feat).await?;
    Ok(value)
}

async fn disable_feature<'a, 'b>(inav: &'a INavMsp, value: &'b str) -> Result<&'b str, &'a str> {
    let index = FEATURE_NAMES.iter().position(|&n| n == value).unwrap();
    let mut feat = inav.get_features().await.unwrap();

    feat.features[index] = false;

    inav.set_features(feat).await?;
    Ok(value)
}

async fn upload_features<'a, 'b>(inav: &'a INavMsp, values:Vec<&str>) -> Result<(), &'a str> {
    let mut feat = inav.get_features().await.unwrap();

    for v in values {
        if &v[..1] == "-" {
            let index = FEATURE_NAMES.iter().position(|&n| n == &v[1..]).unwrap();
            feat.features[index] = false;
        } else {
            let index = FEATURE_NAMES.iter().position(|&n| n == v).unwrap();
            feat.features[index] = true;
        }
    }

    inav.set_features(feat).await?;
    Ok(())
}

async fn dump_feature(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let features = inav.get_features().await?;
    let dump: Vec<String> = features
        .features
        .iter()
        .enumerate()
        .fold(vec![], |mut acc, (i, &is_enabled)| {
            if is_enabled {
                acc.push(format!("{}", FEATURE_NAMES[i]));
            }
            acc
        });

    return Ok(dump);
}

async fn upload_common_settings<'a>(inav: &'a INavMsp, values: Vec<String>, strict: bool) -> Result<(), &'a str> {
    println!("describing settings");
    let setting_list = describe_settings(&inav).await.unwrap();

    let setting_list_key_vals = setting_list
        .iter()
        .enumerate()
        .fold(HashMap::new(), |mut acc, (i, s) | {
            acc.insert(s.name.to_owned(), (i as u16, s));
            acc
        });

    // parse all values to (name, val)
    let set_settings_list = values.iter().map(|v| {
        println!("parsing {}", v);
        let mut split_iter = v.splitn(2, '=');
        let name = split_iter.next().unwrap().to_string().trim().to_owned();
        let val = split_iter.next().unwrap().to_string().trim().to_owned();
        (name, val)
    }).collect::<Vec<(String, String)>>();


    let id_buf_valus_res = set_settings_list.iter().try_fold(vec![], |mut acc, (name, val)| {
        let (i, s) = match setting_list_key_vals.get(name) {
            Some((i, s)) => (i, s),
            None => {
                eprintln!("unsupported setting {}", &name);
                if strict {
                    return None;
                }
                return Some(acc);
            }
        };

        let buf_val = match setting_to_vec(&s, &val) {
            Ok(buf_val) => buf_val,
            Err(e) => {
                eprintln!("unsupported setting value {} {}", &name, e);
                if strict {
                    return None;
                }
                return Some(acc);
            }
        };

        acc.push((i, buf_val));
        return Some(acc)
    });

    let id_buf_valus = match id_buf_valus_res {
        Some(buf) => buf,
        None => return Err("aborting due to unsupported settings")
    };

    let mut set_setting_futures = id_buf_valus.iter()
        .map(|(i, v)| inav.set_setting_by_id(i, v))
        .collect::<FuturesUnordered<_>>();

    loop {
        match set_setting_futures.next().await {
            Some(Ok(id)) => println!("set {}", setting_list[*id as usize].name),
            Some(Err(e)) => return Err(e),
            None => return Ok(()),
        }
    }
}

async fn dump_common_setting(inav: &INavMsp) -> Result<Vec<String>, &str> {
    let settings = describe_settings(inav).await?;
    let dump: Vec<String> = settings
        .iter()
        .map(|s| format!("{} = {}", &s.name, setting_to_str(&s)))
        .collect();

    return Ok(dump);
}

fn setting_to_str(s: &SettingInfo) -> String {
    return match s.info.setting_type {
        SettingType::VarUint8 => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<u8>());
            let val = u8::from_le_bytes(int_bytes.try_into().unwrap());
            if s.info.setting_mode == SettingMode::ModeLookup {
                return s.enum_names[val as usize].to_string();
            }
            return val.to_string();
        },
        SettingType::VarInt8 => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<i8>());
            return i8::from_le_bytes(int_bytes.try_into().unwrap()).to_string();
        }
        SettingType::VarUint16 => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<u16>());
            return u16::from_le_bytes(int_bytes.try_into().unwrap()).to_string();
        }
        SettingType::VarInt16 => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<i16>());
            return i16::from_le_bytes(int_bytes.try_into().unwrap()).to_string();
        }
        SettingType::VarUint32 => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<u32>());
            return u32::from_le_bytes(int_bytes.try_into().unwrap()).to_string();
        }
        SettingType::VarFloat => {
            let (int_bytes, _rest) = s.value.split_at(std::mem::size_of::<f32>());
            return f32::from_le_bytes(int_bytes.try_into().unwrap()).to_string();
        }
        SettingType::VarString => INavMsp::str_from_u8_nul_utf8(&s.value).unwrap().to_owned(),
    };
}

fn setting_to_vec<'a>(s: &SettingInfo, value: &str) -> Result<Vec<u8>, &'a str> {
    return match s.info.setting_type {
        SettingType::VarUint8 => {
            if s.info.setting_mode == SettingMode::ModeLookup {
                let enum_name = String::from(value).to_uppercase();
                let index = s.enum_names.iter().position(|r| r == &enum_name);
                return match index {
                    Some(i) => Ok((i as u8).to_le_bytes().to_vec()),
                    None => {
                        eprintln!("Failed to find {} in {}", enum_name, s.enum_names.join(","));
                        return Err("Failed to find table value");
                    }
                }
            }


            return match value.parse::<u8>() {
                Ok(val) => Ok(val.to_le_bytes().to_vec()),
                _ => Err("Failed to parse"),
            };
        },
        SettingType::VarInt8 => Ok(value.parse::<i8>().unwrap().to_le_bytes().to_vec()),
        SettingType::VarUint16 => Ok(value.parse::<u16>().unwrap().to_le_bytes().to_vec()),
        SettingType::VarInt16 => Ok(value.parse::<i16>().unwrap().to_le_bytes().to_vec()),
        SettingType::VarUint32 => Ok(value.parse::<u32>().unwrap().to_le_bytes().to_vec()),
        SettingType::VarFloat => Ok(value.parse::<f32>().unwrap().to_le_bytes().to_vec()),
        SettingType::VarString => Ok(value.as_bytes().to_vec()),
    };
}

// TODO: use trait from_string or implement strum
// TODO: and move this to the library
fn baudrate_to_string<'a>(baudrate: &Baudrate) -> Result<String, &'a str> {
    let s = match baudrate {
        Baudrate::BaudAuto => "0",
        Baudrate::Baud1200 => "1200",
        Baudrate::Baud2400 => "2400",
        Baudrate::Baud4800 => "4800",
        Baudrate::Baud9600 => "9600",
        Baudrate::Baud19200 => "19200",
        Baudrate::Baud38400 => "38400",
        Baudrate::Baud57600 => "57600",
        Baudrate::Baud115200 => "115200",
        Baudrate::Baud230400 => "230400",
        Baudrate::Baud250000 => "250000",
        Baudrate::Baud460800 => "460800",
        Baudrate::Baud921600 => "921600",
        Baudrate::Baud1000000 => "1000000",
        Baudrate::Baud1500000 => "1500000",
        Baudrate::Baud2000000 => "2000000",
        Baudrate::Baud2470000 => "2470000",
    };

    return Ok(s.to_owned());
}

fn string_to_baudrate<'a>(baudrate_str: &str) -> Result<Baudrate, &'a str> {
    let baudrate = match baudrate_str {
        "0" => Baudrate::BaudAuto,
        "1200" => Baudrate::Baud1200,
        "2400" => Baudrate::Baud2400,
        "4800" => Baudrate::Baud4800,
        "9600" => Baudrate::Baud9600,
        "19200" => Baudrate::Baud19200,
        "38400" => Baudrate::Baud38400,
        "57600" => Baudrate::Baud57600,
        "115200" => Baudrate::Baud115200,
        "230400" => Baudrate::Baud230400,
        "250000" => Baudrate::Baud250000,
        "460800" => Baudrate::Baud460800,
        "921600" => Baudrate::Baud921600,
        "1000000" => Baudrate::Baud1000000,
        "1500000" => Baudrate::Baud1500000,
        "2000000" => Baudrate::Baud2000000,
        "2470000" => Baudrate::Baud2470000,
        _ => return Err("Baudrate not found"),
    };

    return Ok(baudrate);
}

fn u8_to_serial_identifier<'a>(id: u8) -> Result<SerialIdentifier, &'a str> {
    let serial = match id {
        255 => SerialIdentifier::None,
        0 => SerialIdentifier::USART1,
        1 => SerialIdentifier::USART2,
        2 => SerialIdentifier::USART3,
        3 => SerialIdentifier::USART4,
        4 => SerialIdentifier::USART5,
        5 => SerialIdentifier::USART6,
        6 => SerialIdentifier::USART7,
        7 => SerialIdentifier::USART8,
        20 => SerialIdentifier::UsbVcp,
        30 => SerialIdentifier::SoftSerial1,
        31 => SerialIdentifier::SoftSerial2,
        _ => return Err("Serial identifier not found"),
    };

    return Ok(serial);
}
