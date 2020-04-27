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

use std::net::IpAddr;

use reqwest::blocking::ClientBuilder;

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

impl std::fmt::Display for DyfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

const API: &str = "https://www.dy.fi/nic/update";

fn run() -> Result<DyfiResponseCode, DyfiError> {
    env_logger::init();
    dotenv::dotenv().ok();
    let user = dotenv::var("DYFI_USER")?;
    let password = dotenv::var("DYFI_PASSWORD")?;
    let hostnames = dotenv::var("DYFI_HOSTNAMES")?;

    let client = ClientBuilder::new()
        .user_agent("Dyfi-client-rs")
        .build()?;

    let response = client
        .get(API)
        .basic_auth(user, Some(password))
        .query(&[("hostname", hostnames)])
        .send();

    let dyfi_response = DyfiResponse::from(&response?.text()?);
    dyfi_response.log();
    Ok(DyfiResponseCode::from(dyfi_response))
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
