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

use std::net::IpAddr;

pub enum LoopStatus {
    Nop,
    Action(Result<DyfiResponse, DyfiError>),
}

pub struct Config {
    pub dyfi_api: String,
    pub public_ip_api: String,
    pub user: String,
    pub password: String,
    pub hostnames: Vec<String>,
}

#[derive(Debug)]
pub enum DyfiResponse {
    BadAuth,
    NoHost,
    NotFQDN,
    BadIP(IpAddr),
    NoChg,
    Good(IpAddr),
    DNSErr,
    Abuse,
    Other(String)
}

impl DyfiResponse {
    pub fn from(s: String) -> Self {
        let result: Vec<&str> = s.trim().split_whitespace().collect();
        match result[..] {
            ["badauth"] => Self::BadAuth,
            ["nohost"] => Self::NoHost,
            ["notfq"] => Self::NotFQDN,
            ["badip", ..] => Self::BadIP(result[1].parse().unwrap()),
            ["nochg"] => Self::NoChg,
            ["good", ..] => Self::Good(result[1].parse().unwrap()),
            ["dnserr"] => Self::DNSErr,
            ["abuse"] => Self::Abuse,
            _ => Self::Other(s),
        }
    }

    pub fn log(&self) {
        match self {
            Self::BadAuth => error!("dy.fi replied: Authentication failed"),
            Self::NoHost => error!(
                "dy.fi replied: No hostname parameter or hostname not allocated for user"
            ),
            Self::NotFQDN => error!("dy.fi replied: Given hostname not a valid .dy.fi FQDN"),
            Self::BadIP(ip) => error!(
                "dy.fi replied: IP address {} not valid or not registered to a Finnish organisation",
                ip
            ),
            Self::NoChg => info!("dy.fi replied: No change"),
            Self::Good(ip) => info!("dy.fi replied: Hostname(s) pointed at new address {}", ip),
            Self::DNSErr => error!("dy.fi replied: Request failed due to technical problem"),
            Self::Abuse => error!("dy.fi replied: Request denied due to abuse"),
            Self::Other(s) => error!("dy.fi replied with other message: {}", s)
        }
    }
}

#[derive(Debug)]
pub enum DyfiResponseCode {
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

#[derive(Debug)]
pub struct DyfiError(pub String);

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
    fn from(e: std::net::AddrParseError) -> Self {
        DyfiError(format!("Error parsing current IP address: {}", e))
    }
}

impl std::fmt::Display for DyfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
