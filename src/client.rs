// Dyfi-client, a dynamic DNS updater for the dy.fi service.
// Copyright (C) 2020-2023  Ronja Koistinen

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

use crate::types::{Config, DyfiError, DyfiResponse, DyfiResponseCode, LoopStatus};
use crate::{FORCE_UPDATE_INTERVAL};
use reqwest::blocking::ClientBuilder;
use std::collections::HashMap;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};


#[cfg(not(test))]
use crate::LOOP_DELAY;
#[cfg(not(test))]
use std::thread::sleep;

#[inline]
#[cfg(not(test))]
fn do_sleep(duration: u64) {
    debug!("Sleeping {} seconds...", duration);
    sleep(Duration::from_secs(duration));
}

#[inline]
fn resolve_host(host: &str) -> std::io::Result<impl Iterator<Item = IpAddr>> {
    Ok((host, 0).to_socket_addrs()?.map(|x| x.ip()))
}

pub struct Dyfi {
    http_client: reqwest::blocking::Client,
    previous_update_time: Option<Instant>,
    previous_ips: HashMap<String, Vec<IpAddr>>,
    config: Config,
}

impl Dyfi {
    fn do_update(&self) -> Result<DyfiResponse, DyfiError> {
        let http_response = self
            .http_client
            .get(&self.config.dyfi_api)
            .basic_auth(&self.config.user, Some(&self.config.password))
            .query(&[("hostname", &self.config.hostnames.join(","))])
            .send();

        Ok(DyfiResponse::from(http_response?.text()?))
    }

    fn get_current_ip(&self) -> Result<IpAddr, DyfiError> {
        let response = self.http_client.get(&self.config.public_ip_api).send()?;
        if response.status().is_success() {
            match response.text() {
                Ok(text) => match text.trim().parse() {
                    Ok(ip) => Ok(ip),
                    Err(e) => Err(DyfiError(format!("Error parsing current IP: {}", e))),
                },
                Err(e) => Err(DyfiError(format!("Error while fetching current IP: {}", e))),
            }
        } else {
            Err(DyfiError(format!(
                "Error fetching current IP. Server responded with status {}",
                response.status()
            )))
        }
    }

    pub fn from(config: Config) -> Result<Self, DyfiError> {
        debug!("Initializing HTTP client...");
        Ok(Self {
            // init blocking reqwest http client
            http_client: ClientBuilder::new().user_agent("Dyfi-client-rs").build()?,
            previous_update_time: None,
            previous_ips: HashMap::new(),
            config,
        })
    }

    pub fn run(&mut self) -> DyfiResponseCode {
        debug!("Resolving hostname(s)...");
        for host in &self.config.hostnames {
            let ips = match resolve_host(host) {
                Ok(ips) => ips.collect(),
                Err(_) => vec![],
            };
            debug!("{} currently resolves to {:?}", &host, ips);
            self.previous_ips.insert(host.clone(), ips);
        }

        loop {
            debug!(
                "Getting my current IP address from {}",
                self.config.public_ip_api
            );
            let my_ip = match self.get_current_ip() {
                Ok(ip) => ip,
                Err(e) => {
                    // we hit an error checking our current ip address.
                    // log it and try again later.
                    info!("{}", e);
                    #[cfg(test)]
                    break DyfiResponseCode::OtherNonFatal;

                    #[cfg(not(test))]
                    {
                        do_sleep(LOOP_DELAY / 4);
                        continue;
                    }
                }
            };
            debug!("My current IP address is {}", my_ip);

            let dyfi_status: LoopStatus = if let Some(prev_update) = self.previous_update_time {
                if prev_update.elapsed() < Duration::from_secs(FORCE_UPDATE_INTERVAL) {
                    // there is a previous update and it was less than FORCE_UPDATE_INTERVAL ago
                    if self.previous_ips.iter().any(|(_, v)| v.is_empty()) {
                        // any one or several of the hostnames does not have a previous ip
                        info!("No current IP for one or more hostnames, updating...");
                        LoopStatus::Action(self.do_update())
                    } else {
                        // resolve all hostnames configured
                        let mut resolved_ips = self
                            .config
                            .hostnames
                            .iter()
                            .map(|h| resolve_host(h))
                            .filter_map(Result::ok)
                            .flatten();
                        // if any hostname resolves to any ip other than my current ip,
                        // run an update
                        if resolved_ips.any(|ip| ip != my_ip) {
                            info!("One or more hostnames have an outdated IP, updating...");
                            LoopStatus::Action(self.do_update())
                        } else {
                            // all hostnames resolve to my_ip, do nothing
                            debug!("IP address {} is up to date", my_ip);
                            LoopStatus::Nop
                        }
                    }
                } else {
                    // previous update was more than FORCE_UPDATE_INTERVAL ago
                    info!(
                        "More than {} seconds passed, forcing update...",
                        FORCE_UPDATE_INTERVAL
                    );
                    LoopStatus::Action(self.do_update())
                }
            } else {
                // There is no previous update during the runtime of the program.
                // We don't know how long until dy.fi releases the DNS name,
                // so we run an update here.
                info!("No update performed yet, time until release unknown. Updating...");
                LoopStatus::Action(self.do_update())
            };

            match dyfi_status {
                LoopStatus::Action(Ok(response)) => {
                    match response {
                        // New IP has been set. Log it. Set previous_ip and previous_update_time.
                        DyfiResponse::Good(new_ip) => {
                            response.log();
                            self.previous_ips
                                .iter_mut()
                                .for_each(|(_, val)| *val = vec![new_ip]);
                            self.previous_update_time = Some(Instant::now());
                        }
                        // No change. Log it. Set previous_update_time.
                        DyfiResponse::NoChg => {
                            response.log();
                            self.previous_update_time = Some(Instant::now());
                        }
                        // Dy.fi returned a bad status. Log it and break the program loop.
                        _ => {
                            response.log();
                            error!("Unrecoverable error, exiting...");
                            break DyfiResponseCode::from(response);
                        }
                    }
                }
                // do_update() returned an error. This is probably a temporary
                // HTTP error. Log it.
                LoopStatus::Action(Err(e)) => {
                    error!("{}", e);
                }
                LoopStatus::Nop => (),
            }
            #[cfg(test)]
            break DyfiResponseCode::Ok;

            #[cfg(not(test))]
            // Sleep for LOOP_DELAY seconds.
            do_sleep(LOOP_DELAY);
        }
    }
}
