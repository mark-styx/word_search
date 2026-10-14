use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::collections::HashMap;

const SERVICE_TYPE: &str = "_wordsearch._tcp.local.";

pub struct MdnsAdvertiser {
    daemon: ServiceDaemon,
    service_name: String,
}

impl MdnsAdvertiser {
    pub fn new(port: u16) -> Result<Self, Box<dyn std::error::Error>> {
        let daemon = ServiceDaemon::new()?;

        let hostname = hostname::get()
            .ok()
            .and_then(|h| h.into_string().ok())
            .unwrap_or_else(|| "unknown".into());

        let service_name = format!("Word Search ({})", hostname);

        let mut properties: HashMap<String, String> = HashMap::new();
        properties.insert("version".into(), "1.0".into());
        properties.insert("game".into(), "word-search".into());
        properties.insert("path".into(), "/login".into());
        properties.insert("frontend_port".into(), std::env::var("FRONTEND_PORT").unwrap_or_else(|_| "3089".into()));

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            &service_name,
            &format!("{}.local.", hostname.to_lowercase()),
            "",
            port,
            properties,
        )?;

        daemon.register(service_info)?;

        tracing::info!("mDNS service registered: {} on port {}", SERVICE_TYPE, port);

        Ok(Self {
            daemon,
            service_name,
        })
    }

    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.daemon
            .unregister(&format!("{}.{}", self.service_name, SERVICE_TYPE))?;
        tracing::info!("mDNS service unregistered");
        Ok(())
    }
}

impl Drop for MdnsAdvertiser {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
