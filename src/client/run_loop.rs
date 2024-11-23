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

use super::Dyfi;
use crate::types::{DyfiResponse, DyfiResponseCode, LoopStatus};
use crate::FORCE_UPDATE_INTERVAL;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

#[cfg(not(test))]
use crate::LOOP_DELAY;
#[cfg(not(test))]
use std::thread::sleep;

#[inline]
#[cfg(not(test))]
fn do_sleep(secs: u64) {
    debug!("Sleeping {} seconds...", secs);
    sleep(Duration::from_secs(secs));
}

#[inline]
fn resolve_host(host: &str) -> std::io::Result<impl Iterator<Item = IpAddr>> {
    Ok((host, 0).to_socket_addrs()?.map(|x| x.ip()))
}

impl Dyfi {
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
            self.my_ip = match self.get_current_ip() {
                Ok(ip) => Some(ip),
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
            if let Some(ip) = self.my_ip {
                debug!("My current IP address is {ip}");
            } else {
                debug!("My current IP is unknown");
            }

            let dyfi_status = self.resolve_status();

            match dyfi_status {
                LoopStatus::Action(Ok(response)) => {
                    if let Err(e) = self.handle_ok_response(response) {
                        break e;
                    }
                }
                // do_update() returned an error. This is probably a temporary
                // HTTP error.
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

    #[inline]
    fn refresh_update_time(&mut self) {
        self.previous_update_time = Some(Instant::now());
    }

    /// A command has been successfully sent to dy.fi and a response has been
    /// received. This function handles the response, which can be a success
    /// or an error.
    fn handle_ok_response(
        &mut self,
        res: DyfiResponse,
    ) -> Result<(), DyfiResponseCode> {
        res.log();
        match res {
            // New IP has been set.
            // Set previous_ip and previous_update_time.
            DyfiResponse::Good(Some(new_ip)) => {
                self.previous_ips
                    .iter_mut()
                    .for_each(|(_, val)| *val = vec![new_ip]);
                self.refresh_update_time();
            }
            // No change. Set previous_update_time.
            DyfiResponse::NoChg => {
                self.refresh_update_time();
            }
            // Dy.fi returned a bad status.
            // Log it and break the program loop.
            _ => {
                error!("Unrecoverable error, exiting...");
                return Err(DyfiResponseCode::from(res));
            }
        }
        Ok(())
    }

    /// Decide what action is needed on this iteration
    fn resolve_status(&mut self) -> LoopStatus {
        let force_time = Duration::from_secs(FORCE_UPDATE_INTERVAL);
        let current_ip = self.my_ip;
        let mut must_update = false;
        if self
            .previous_update_time
            .is_some_and(|x| x.elapsed() < force_time)
        {
            for (host, ips) in &mut self.previous_ips {
                if ips.is_empty() {
                    // This means the dy.fi DNS service doesn't know about this
                    // host and we need to tell it by running an update
                    info!("No current IP for {host}, updating...");
                    // ret_status = LoopStatus::Action(self.do_update());
                    must_update = true;
                }
                match resolve_host(host) {
                    Ok(new_ips) => {
                        *ips = new_ips.collect();
                    }
                    Err(e) => {
                        error!("Unable to resolve host {host}: {e}");
                        must_update = true;
                    }
                }
                if let Some(curr_ip) = current_ip {
                    if let Some(ip) = ips.iter_mut().find(|ip| **ip != curr_ip)
                    {
                        info!("Host {host} has outdated ip {ip}, updating...");
                        must_update = true;
                    }
                }
            }
        } else {
            info!("Too long since last update or no updates yet. Updating...");
            must_update = true;
        }
        if must_update {
            LoopStatus::Action(self.do_update())
        } else {
            LoopStatus::Nop
        }
    }
}
