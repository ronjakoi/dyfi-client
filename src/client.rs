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

use crate::types::{Config, DyfiError, DyfiResponse, Hostname};
use reqwest::blocking::ClientBuilder;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

mod run_loop;

pub struct Dyfi {
    http_client: reqwest::blocking::Client,
    previous_update_time: Option<Instant>,
    previous_ips: HashMap<Hostname, Vec<IpAddr>>,
    config: Config,
    my_ip: Option<IpAddr>,
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
        let response =
            self.http_client.get(&self.config.public_ip_api).send()?;
        if response.status().is_success() {
            match response.text() {
                Ok(text) => match text.trim().parse() {
                    Ok(ip) => Ok(ip),
                    Err(e) => {
                        Err(DyfiError(format!("Error parsing current IP: {e}")))
                    }
                },
                Err(e) => Err(DyfiError(format!(
                    "Error while fetching current IP: {e}"
                ))),
            }
        } else {
            Err(DyfiError(format!(
                "Error fetching current IP. Server responded with status {}",
                response.status()
            )))
        }
    }

    pub fn from(config: Config) -> Result<Self, DyfiError> {
        if config.hostnames.is_empty() {
            return Err(DyfiError("No hostnames configured".to_string()));
        }
        debug!("Initializing HTTP client...");
        Ok(Self {
            // init blocking reqwest http client
            http_client: ClientBuilder::new()
                .user_agent("Dyfi-client-rs")
                .build()?,
            previous_update_time: None,
            previous_ips: HashMap::new(),
            config,
            my_ip: None,
        })
    }
}
