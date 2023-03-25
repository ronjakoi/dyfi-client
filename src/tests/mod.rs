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

#[cfg(test)]

use crate::client::Dyfi;
use crate::types::Config;
use crate::types::DyfiResponseCode;
use mockito::{Matcher, Mock};
use std::env;
use std::sync::Once;

static INIT: Once = Once::new();
const MOCK_IP: &str = "192.0.2.1"; // RFC 5737

fn log_init() {
    env::set_var("RUST_LOG", "dyfi_client=debug");
    INIT.call_once(|| {
        env_logger::init();
    });
}

struct TestServer {
    server: mockito::ServerGuard,
}

impl TestServer {
    pub fn new() -> Self {
        TestServer {
            server: mockito::Server::new()
        }
    }

    pub fn make_test_config(&self) -> Config {
        Config {
            dyfi_api: format!("{}{}", self.server.url(), "/nic/update"),
            public_ip_api: self.server.url(),
            user: String::from("mockuser"),
            password: String::from("mockpassword"),
            hostnames: vec![
                String::from("mock.dy.fi"),
                String::from("mock-some-more.dy.fi"),
            ],
        }
    }

    pub fn dyfi_mock_base(&mut self) -> Mock {
        self.server.mock("GET", "/nic/update")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .match_query(Matcher::UrlEncoded(
                "hostname".to_string(),
                "mock.dy.fi,mock-some-more.dy.fi".to_string(),
            ))
            .expect(1)
    }

    fn get_ip_mock(&mut self) -> Mock {
        self.server.mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(MOCK_IP)
            .expect(1)
            .create()
    }
}


#[test]
fn test_update_nocfg() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base().with_body("nochg").create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::Ok);
}

#[test]
fn test_update_badauth() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base().with_body("badauth").create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::BadAuth);
}

#[test]
fn test_update_nohost() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base()
        .match_query(Matcher::AnyOf(vec![
            Matcher::Missing,
            Matcher::Regex("".to_string()),
            Matcher::Regex("hostname=".to_string()),
        ]))
        .with_body("nohost")
        .create();
    let mut config = server.make_test_config();
    config.hostnames = vec![];
    let code = Dyfi::from(config).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::NoHost);
}

#[test]
fn test_update_notfqdn() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base()
        .match_query(Matcher::UrlEncoded(
            "hostname".to_string(),
            "something-outrageous,example.com".to_string(),
        ))
        .with_body("notfqdn")
        .create();
    let mut config = server.make_test_config();
    config.hostnames = vec![
        "something-outrageous".to_string(),
        "example.com".to_string(),
    ];
    let code = Dyfi::from(config).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::NotFQDN);
}

#[test]
fn test_update_badip() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base()
        .with_body(format!("badip {}", MOCK_IP))
        .create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::BadIP);
}

#[test]
fn test_update_good() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base()
        .with_body(format!("good {}", MOCK_IP))
        .create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::Ok);
}

#[test]
fn test_update_dnserr() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base().with_body("dnserr").create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::DNSErr);
}

#[test]
fn test_update_abuse() {
    log_init();
    let mut server = TestServer::new();
    let get_ip = server.get_ip_mock();
    let dyfi_nocfg = server.dyfi_mock_base().with_body("abuse").create();
    let code = Dyfi::from(server.make_test_config()).unwrap().run();
    get_ip.assert();
    dyfi_nocfg.assert();
    dyfi_nocfg.matched();
    assert_eq!(code, DyfiResponseCode::Abuse);
}
