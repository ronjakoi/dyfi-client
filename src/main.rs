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

mod types;
use types::{Config, LoopStatus, DyfiError, DyfiResponse, DyfiResponseCode};

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
    env_logger::init();

    // init blocking reqwest http client
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

        let dyfi_status: LoopStatus = match previous_update_time {
            Some(prev_update) => {
                if Instant::now() - prev_update < Duration::from_secs(FORCE_UPDATE_INTERVAL) {
                    // there is a previous update and it was less than FORCE_UPDATE_INTERVAL ago
                    if previous_ip.is_empty() {
                        // the hostname does not currently resolve to any IP
                        info!("No current IP for hostnames, updating...");
                        LoopStatus::Action(do_update(&client, &config))
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
                            LoopStatus::Action(do_update(&client, &config))
                        } else {
                            // all IP addresses resolve to my_ip, do nothing
                            debug!("IP address {} is up to date", my_ip);
                            LoopStatus::Nop
                        }
                    }
                } else {
                    // previous update was more than FORCE_UPDATE_INTERVAL ago
                    info!("More than {} seconds passed, forcing update...", FORCE_UPDATE_INTERVAL);
                    LoopStatus::Action(do_update(&client, &config))
                }
            },
            None => {
                // There is no previous update during the runtime of the program.
                // We don't know how long until dy.fi releases the DNS name,
                // so we run an update here.
                info!("No update performed yet, time until release unknown. Updating...");
                LoopStatus::Action(do_update(&client, &config))
            }
        };

        match dyfi_status {
            LoopStatus::Action(Ok(response)) => {
                match response {
                    // New IP has been set. Log it. Set previous_ip and previous_update_time.
                    DyfiResponse::Good(new_ip) => {
                        response.log();
                        previous_ip = vec![new_ip];
                        previous_update_time = Some(Instant::now());
                    },
                    // No change. Log it. Set previous_update_time.
                    DyfiResponse::NoChg => {
                        response.log();
                        previous_update_time = Some(Instant::now());
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
            // HTTP error. Log it.
            LoopStatus::Action(Err(e)) => {
                error!("{}", e);
            },
            LoopStatus::Nop => (),
        }
        // Sleep for LOOP_DELAY seconds.
        do_sleep();
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
