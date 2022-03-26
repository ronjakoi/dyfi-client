// Dyfi-client, a dynamic DNS updater for the dy.fi service.
// Copyright (C) 2022  Ronja Koistinen

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

#![warn(clippy::pedantic)]

#[macro_use]
extern crate log;

#[cfg(test)]
mod tests;

mod types;
mod client;
use types::Config;
use client::Dyfi;

const DEFAULT_PUBLIC_IP_API: &str = "http://checkip.amazonaws.com/";
const DEFAULT_DYFI_API: &str = "https://www.dy.fi/nic/update";
const LOOP_DELAY: u64 = 3600; // seconds
const FORCE_UPDATE_INTERVAL: u64 = 3600 * 24 * 5;

fn main() {
    env_logger::init();
    debug!("Reading configuration from environment...");
    dotenv::dotenv().ok();

    let config = Config {
        dyfi_api: dotenv::var("DYFI_API").unwrap_or_else(|_| DEFAULT_DYFI_API.to_string()),
        public_ip_api: dotenv::var("PUBLIC_IP_API").unwrap_or_else(|_| DEFAULT_PUBLIC_IP_API.to_string()),
        user: dotenv::var("DYFI_USER").expect("DYFI_USERNAME not set"),
        password: dotenv::var("DYFI_PASSWORD").expect("DYFI_PASSWORD not set"),
        hostnames: dotenv::var("DYFI_HOSTNAMES").expect("DYFI_HOSTNAMES not set")
            .split(',')
            .map(std::string::ToString::to_string)
            .collect(),
    };
    let mut dyfi = match Dyfi::from(config) {
        Ok(dyfi) => dyfi,
        Err(e) => {
            error!("Error initializing client: {}", e);
            std::process::exit(10);
        }
    };

    std::process::exit(dyfi.run() as i32)
}
