use crate::network::{do_update, get_current_ip, resolve_host};
use crate::types::{Config, DyfiError, DyfiResponse, DyfiResponseCode, LoopStatus};
use crate::{FORCE_UPDATE_INTERVAL, LOOP_DELAY};
use reqwest::blocking::ClientBuilder;
use std::collections::HashMap;
use std::net::IpAddr;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[inline]
fn do_sleep(duration: u64) {
    debug!("Sleeping {} seconds...", duration);
    sleep(Duration::from_secs(duration));
}

pub fn run(config: Config) -> Result<DyfiResponseCode, DyfiError> {
    // init blocking reqwest http client
    debug!("Initializing HTTP client...");
    let client = ClientBuilder::new().user_agent("Dyfi-client-rs").build()?;

    let mut previous_update_time: Option<Instant> = None;
    let mut previous_ips: HashMap<String, Vec<IpAddr>> = HashMap::new();
    debug!("Resolving hostname(s)...");
    for host in &config.hostnames {
        let ips = match resolve_host(host) {
            Ok(ips) => ips.collect(),
            Err(_) => vec![],
        };
        debug!("{} currently resolves to {:?}", &host, ips);
        previous_ips.insert(host.clone(), ips);
    }

    Ok(loop {
        debug!(
            "Getting my current IP address from {}",
            config.public_ip_api
        );
        let get_my_ip = get_current_ip(&client, &config.public_ip_api);
        let my_ip = match get_my_ip {
            Ok(ip) => ip,
            Err(e) => {
                // we hit an error checking our current ip address.
                // log it and try again later.
                info!("{}", e);
                #[cfg(test)]
                break DyfiResponseCode::Ok;

                #[cfg(not(test))] {
                    do_sleep(LOOP_DELAY / 4);
                    continue;
                }
            }
        };
        debug!("My current IP address is {}", my_ip);

        let dyfi_status: LoopStatus = match previous_update_time {
            Some(prev_update) => {
                if prev_update.elapsed() < Duration::from_secs(FORCE_UPDATE_INTERVAL) {
                    // there is a previous update and it was less than FORCE_UPDATE_INTERVAL ago
                    if previous_ips.iter().any(|(_, v)| v.is_empty()) {
                        // any one or several of the hostnames does not have a previous ip
                        info!("No current IP for one or more hostnames, updating...");
                        LoopStatus::Action(do_update(&client, &config))
                    } else {
                        // resolve all hostnames configured
                        let mut resolved_ips = config
                            .hostnames
                            .iter()
                            .map(|h| resolve_host(h))
                            .filter_map(Result::ok)
                            .flatten();
                        // if any hostname resolves to any ip other than my current ip,
                        // run an update
                        if resolved_ips.any(|ip| ip != my_ip) {
                            info!("One or more hostnames have an outdated IP, updating...");
                            LoopStatus::Action(do_update(&client, &config))
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
                    LoopStatus::Action(do_update(&client, &config))
                }
            }
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
                        previous_ips
                            .iter_mut()
                            .for_each(|(_, val)| *val = vec![new_ip]);
                        previous_update_time = Some(Instant::now());
                    }
                    // No change. Log it. Set previous_update_time.
                    DyfiResponse::NoChg => {
                        response.log();
                        previous_update_time = Some(Instant::now());
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
    })
}
