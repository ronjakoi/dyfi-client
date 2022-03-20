use crate::types::{Config, DyfiError, DyfiResponse};
use reqwest::IntoUrl;
use std::net::{IpAddr, ToSocketAddrs};

// Resolve a hostname given as string to an IP address or several.
// Returns a Result containing an Ok with an iterator of IP addresses,
// or an Err if host resolution failed.
pub fn resolve_host(host: &str) -> std::io::Result<impl Iterator<Item = IpAddr>> {
    Ok((host, 0).to_socket_addrs()?.map(|x| x.ip()))
}

pub fn do_update(
    client: &reqwest::blocking::Client,
    config: &Config,
) -> Result<DyfiResponse, DyfiError> {
    let http_response = client
        .get(&config.dyfi_api)
        .basic_auth(&config.user, Some(&config.password))
        .query(&[("hostname", &config.hostnames.join(","))])
        .send();

    Ok(DyfiResponse::from(http_response?.text()?))
}

pub fn get_current_ip<U>(client: &reqwest::blocking::Client, api: U) -> Result<IpAddr, DyfiError>
where
    U: IntoUrl,
{
    let response = client.get(api).send()?;
    if !response.status().is_success() {
        Err(DyfiError(format!(
            "Error fetching current IP. Server responded with status {}",
            response.status()
        )))
    } else {
        match response.text() {
            Ok(text) => match text.trim().parse() {
                Ok(ip) => Ok(ip),
                Err(e) => Err(DyfiError(format!("Error parsing current IP: {}", e))),
            },
            Err(e) => Err(DyfiError(format!("Error while fetching current IP: {}", e))),
        }
    }
}
