// Dyfi-client, a dynamic DNS updater for the dy.fi service.
// Copyright (C) 2020  Ronja Koistinen

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[macro_use]
extern crate log;

use std::net::{IpAddr, ToSocketAddrs};
use std::thread::sleep;
use std::time::{Instant, Duration};

use reqwest::blocking::ClientBuilder;

struct Config {
    pub user: String,
    pub password: String,
    pub hostnames: Vec<String>,
}

#[derive(Debug)]
enum DyfiResponse {
    BadAuth,
    NoHost,
    NotFQDN,
    BadIP(IpAddr),
    NoChg,
    Good(IpAddr),
    DNSErr,
    Abuse,
}

impl DyfiResponse {
    pub fn from(s: &str) -> Self {
        let s: Vec<&str> = s.trim().split_whitespace().collect();
        match s[..] {
            ["badauth"] => Self::BadAuth,
            ["nohost"] => Self::NoHost,
            ["notfq"] => Self::NotFQDN,
            ["badip", ..] => Self::BadIP(s[1].parse().unwrap()),
            ["nochg"] => Self::NoChg,
            ["good", ..] => Self::Good(s[1].parse().unwrap()),
            ["dnserr"] => Self::DNSErr,
            ["abuse"] => Self::Abuse,
            _ => unreachable!(),
        }
    }

    pub fn log(&self) {
        match self {
            Self::BadAuth => error!("Authentication failed"),
            Self::NoHost => error!("No hostname parameter or hostname not allocated for user"),
            Self::NotFQDN => error!("Given hostname not a valid .dy.fi FQDN"),
            Self::BadIP(ip) => error!(
                "IP address {} not valid or not registered to a Finnish organisation",
                ip
            ),
            Self::NoChg => info!("No change"),
            Self::Good(ip) => info!("Hostname(s) pointed at new address {}", ip),
            Self::DNSErr => error!("Request failed due to technical problem at dy.fi"),
            Self::Abuse => error!("Request denied due to abuse"),
        }
    }
}

enum DyfiResponseCode {
    BadAuth = 1,
    NoHost = 2,
    NotFQDN = 3,
    BadIP = 4,
    Ok = 0,
    DNSErr = 5,
    Abuse = 6,
}

impl From<DyfiResponse> for DyfiResponseCode {
    fn from(d: DyfiResponse) -> Self {
        match d {
            DyfiResponse::BadAuth => Self::BadAuth,
            DyfiResponse::NoHost => Self::NoHost,
            DyfiResponse::NotFQDN => Self::NotFQDN,
            DyfiResponse::BadIP(_) => Self::BadIP,
            DyfiResponse::DNSErr => Self::DNSErr,
            DyfiResponse::Abuse => Self::Abuse,
            _ => Self::Ok,
        }
    }
}

struct DyfiError(String);

impl From<dotenv::Error> for DyfiError {
    fn from(e: dotenv::Error) -> Self {
        DyfiError(e.to_string())
    }
}

impl From<reqwest::Error> for DyfiError {
    fn from(e: reqwest::Error) -> Self {
        DyfiError(e.to_string())
    }
}

impl From<std::net::AddrParseError> for DyfiError {
    fn from(_e: std::net::AddrParseError) -> Self {
        DyfiError(format!("Error parsing current IP address returned from {}", PUBLIC_IP_API))
    }
}

impl std::fmt::Display for DyfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Resolve a hostname given as string to an IP address or several.
// Returns a Result containing an Ok with an iterator of IP addresses,
// or an Err if host resolution failed.
fn resolve_host(host: &str) -> std::io::Result<impl Iterator<Item = IpAddr>> {
    Ok(
        (host, 0)
        .to_socket_addrs()?
        .map(|x| x.ip())
    )
}

fn do_update(client: &reqwest::blocking::Client, config: &Config) -> Result<DyfiResponse, DyfiError> {
    let http_response = client
        .get(DYFI_API)
        .basic_auth(&config.user, Some(&config.password))
        .query(&[("hostname", &config.hostnames.join(","))])
        .send();

    Ok(DyfiResponse::from(&http_response?.text()?))
}

#[inline]
fn do_sleep() {
    debug!("Sleeping {} seconds...", LOOP_DELAY);
    sleep(Duration::from_secs(LOOP_DELAY));
}

fn get_current_ip(client: &reqwest::blocking::Client) -> Result<IpAddr, DyfiError> {
    let response = client
        .get(PUBLIC_IP_API)
        .send()?;
    let status = response.status();
    if !status.is_success() {
        Err(DyfiError(format!("Error fetching current IP. Server responded with status {}", status)))
    } else {
        match response.text() {
            Ok(text) => match text.trim().parse() {
                Ok(ip) => Ok(ip),
                Err(e) => Err(
                    DyfiError(format!("Error parsing current IP: {}", e))
                )
            },
            Err(e) => Err(
                DyfiError(format!("Error while fetching current IP: {}", e))
            ),
        }
    }
}

const PUBLIC_IP_API: &str = "http://checkip.amazonaws.com/";
const DYFI_API: &str = "https://www.dy.fi/nic/update";
const LOOP_DELAY: u64 = 3600; // seconds
const FORCE_UPDATE_INTERVAL: u64 = 3600 * 24 * 5;

fn run() -> Result<DyfiResponseCode, DyfiError> {
    dotenv::dotenv().unwrap();
    let config = Config {
        user: dotenv::var("DYFI_USER")?,
        password: dotenv::var("DYFI_PASSWORD")?,
        hostnames: dotenv::var("DYFI_HOSTNAMES")?
            .split(',')
            .map(|x| x.to_string())
            .collect(),
    };
    std::env::set_var("RUST_LOG", dotenv::var("RUST_LOG")?);
    env_logger::init();

    let client = ClientBuilder::new()
        .user_agent("Dyfi-client-rs")
        .build()?;

    let mut previous_update_time: Option<Instant> = None;
    let mut previous_ip: Vec<_> = match resolve_host(&config.hostnames[0]) {
        Err(_) => vec![],
        Ok(ips) => ips.collect(),
    };

    Ok(loop {
        debug!("Getting current IP address from {}", PUBLIC_IP_API);
        let my_ip: IpAddr = get_current_ip(&client)?;
        debug!("Current IP address is {}", my_ip);

        let dyfi_status: Option<Result<DyfiResponse, DyfiError>> =
            match previous_update_time {
                Some(prev_update) => {
                    if Instant::now() - prev_update < Duration::from_secs(FORCE_UPDATE_INTERVAL) {
                        // there is a previous update and it was less than FORCE_UPDATE_INTERVAL ago
                        if previous_ip.is_empty() {
                            // the hostname does not currently resolve to any IP
                            info!("No current IP for hostnames, updating...");
                            Some(do_update(&client, &config))
                        } else {
                            // resolve all hostnames configured
                            let mut current_ips = config.hostnames
                                .iter()
                                .map(|h| resolve_host(h))
                                .filter_map(Result::ok)
                                .flatten();
                            // if any hostname resolves to any ip other than the previous known ip,
                            // run an update
                            if current_ips.any(|ip| ip != my_ip) {
                                info!("Hostname currently resolves to outdated IP, updating...");
                                Some(do_update(&client, &config))
                            } else {
                                // all IP addresses resolve to my_ip, do nothing
                                debug!("IP address {} up-to-date.", my_ip);
                                None
                            }
                        }
                    } else {
                        // previous update was more than FORCE_UPDATE_INTERVAL ago
                        info!("More than {} seconds passed, forcing update...", FORCE_UPDATE_INTERVAL);
                        Some(do_update(&client, &config))
                    }
                },
                None => {
                    // There is no previous update during the runtime of the program.
                    // We don't know how long until dy.fi releases the DNS name,
                    // so we run an update here.
                    info!("No update performed yet, time until release unknown. Updating...");
                    Some(do_update(&client, &config))
                }
            };

        match dyfi_status {
            Some(Ok(response)) => {
                match response {
                    // New IP has been set. Log it. Set previous_ip and previous_update_time.
                    // Sleep.
                    DyfiResponse::Good(new_ip) => {
                        response.log();
                        previous_ip = vec![new_ip];
                        previous_update_time = Some(Instant::now());
                        do_sleep();
                    },
                    // No change. Log it. Set previous_update_time and sleep.
                    DyfiResponse::NoChg => {
                        response.log();
                        previous_update_time = Some(Instant::now());
                        do_sleep();
                    },
                    // Dy.fi returned a bad status. Log it and break the program loop.
                    _ => {
                        response.log();
                        error!("Unrecoverable error, exiting...");
                        break DyfiResponseCode::from(response);
                    }
                }
            },
            // do_update() returned an error. This is probably a temporary
            // HTTP error. Log it and sleep.
            Some(Err(e)) => {
                error!("{}", e);
                do_sleep();
            },
            _ => {},
        }
    })
}

fn main() {
    std::process::exit(match run() {
        Ok(res) => res as i32,
        Err(err) => {
            error!("{}", err);
            10 // initialization error from dotenv or reqwest
        }
    })
}
