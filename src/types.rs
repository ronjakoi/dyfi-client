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

use std::net::IpAddr;

pub type Hostname = String;

pub enum LoopStatus {
    Nop,
    Action(Result<DyfiResponse, DyfiError>),
}

#[derive(Debug)]
pub struct Config {
    pub dyfi_api: String,
    pub public_ip_api: String,
    pub user: String,
    pub password: String,
    pub hostnames: Vec<Hostname>,
}

#[derive(Debug)]
pub enum DyfiResponse {
    BadAuth,
    NoHost,
    NotFQDN,
    BadIP(IpAddr),
    NoChg,
    /// The request was valid and processed successfully, and caused
    /// the hostname to be pointed to the IP address returned.
    /// If this was was an 'offline' request, the response does not contain
    /// the IP address.
    Good(Option<IpAddr>),
    /// The request failed due to a technical problem at the dy.fi service.
    DNSErr,
    Abuse,
    Other(String),
}

impl DyfiResponse {
    pub fn from(s: String) -> Self {
        let result: Vec<&str> = s.split_whitespace().collect();
        match result[..] {
            ["badauth"] => Self::BadAuth,
            ["nohost"] => Self::NoHost,
            ["notfqdn"] => Self::NotFQDN,
            ["badip", ip] => Self::BadIP(ip.parse().unwrap()),
            ["nochg"] => Self::NoChg,
            ["good", ip] => Self::Good(Some(ip.parse().unwrap())),
            ["good"] => {
                // The Good response with no IP address is sent to an `offline`
                // command which releases the IP address from the service.
                // This program does not support this command.
                unimplemented!()
            }
            ["dnserr"] => Self::DNSErr,
            ["abuse"] => Self::Abuse,
            _ => Self::Other(s),
        }
    }

    pub fn log(&self) {
        match self {
            Self::BadAuth => error!("dy.fi replied: Authentication failed"),
            Self::NoHost => error!(concat!(
                "dy.fi replied: No hostname parameter or hostname ",
                "not allocated for user"
            )),
            Self::NotFQDN => {
                error!("dy.fi replied: Given hostname not a valid .dy.fi FQDN");
            }
            Self::BadIP(ip) => error!(
                concat!(
                    "dy.fi replied: IP address {} not valid or not registered ",
                    "to a Finnish organisation"
                ),
                ip
            ),
            Self::NoChg => info!("dy.fi replied: No change"),
            Self::Good(Some(ip)) => {
                info!("dy.fi replied: Hostname(s) pointed at new address {ip}");
            }
            Self::Good(None) => {
                unimplemented!()
            }
            Self::DNSErr => {
                error!(
                    "dy.fi replied: Request failed due to technical problem"
                );
            }
            Self::Abuse => error!("dy.fi replied: Request denied due to abuse"),
            Self::Other(s) => error!("dy.fi replied with other message: '{s}'"),
        }
    }
}

#[derive(Debug, PartialEq)]
#[rustfmt::skip]
#[repr(i32)]
pub enum DyfiResponseCode {
    // These are from the dy.fi API
    BadAuth       = 1,
    NoHost        = 2,
    NotFQDN       = 3,
    BadIP         = 4,
    Ok            = 0,
    DNSErr        = 5,
    Abuse         = 6,
    // This one is not
    #[cfg(test)]
    OtherNonFatal = 99,
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

impl From<dotenvy::Error> for DyfiError {
    fn from(e: dotenvy::Error) -> Self {
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
        DyfiError(format!("Error parsing current IP address: {e}"))
    }
}

impl std::fmt::Display for DyfiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
