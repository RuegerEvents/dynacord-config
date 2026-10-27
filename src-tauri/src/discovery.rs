//! Browses mDNS for `_oca._tcp` (AES70 over TCP) and keeps a live list of devices.

use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Serialize)]
pub struct Device {
    pub name: String,
    pub hostname: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub serial: Option<String>,
    pub model_guid: Option<String>,
}

#[derive(Clone, Default)]
pub struct Registry(Arc<Mutex<BTreeMap<String, Device>>>);

impl Registry {
    pub fn list(&self) -> Vec<Device> {
        self.0.lock().unwrap().values().cloned().collect()
    }

    pub fn start(&self) {
        let daemon = match ServiceDaemon::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!("mDNS unavailable: {e}");
                return;
            }
        };
        let rx = daemon.browse("_oca._tcp.local.").expect("mDNS browse");
        let map = self.0.clone();
        std::thread::spawn(move || {
            let _daemon = daemon; // keep alive
            while let Ok(event) = rx.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let mut addresses: Vec<String> = info
                            .get_addresses()
                            .iter()
                            .filter(|a| a.is_ipv4())
                            .map(|a| a.to_string())
                            .collect();
                        addresses.sort();
                        let name = info
                            .get_fullname()
                            .split("._oca")
                            .next()
                            .unwrap_or_default()
                            .to_string();
                        let dev = Device {
                            name,
                            hostname: info.get_hostname().trim_end_matches('.').to_string(),
                            addresses,
                            port: info.get_port(),
                            serial: info.get_property_val_str("serialNumber").map(String::from),
                            model_guid: info.get_property_val_str("modelGUID").map(String::from),
                        };
                        map.lock().unwrap().insert(info.get_fullname().to_string(), dev);
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        map.lock().unwrap().remove(&fullname);
                    }
                    _ => {}
                }
            }
        });
    }
}
