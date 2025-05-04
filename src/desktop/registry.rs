use std::collections::HashMap;

use zbus::zvariant::OwnedValue;

use crate::{proxy::Proxy, Error, PortalError};

use super::{HandleToken, Request};

#[derive(Debug)]
struct RegistryProxy<'a>(Proxy<'a>);

impl<'a> RegistryProxy<'a> {
    pub async fn new() -> Result<RegistryProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.host.portal.Registry").await?;
        Ok(Self(proxy))
    }

    pub async fn register(
        &self,
        app_id: &str,
        _options: HashMap<String, OwnedValue>,
    ) -> Result<(), Error> {
        self.0
            .call_method("Register", &(app_id, _options))
            .await
            .map_err::<PortalError, _>(From::from)?;

        Ok(())
    }
}

#[derive(Debug, Default)]
#[doc(alias = "org.freedesktop.host.portal.Registry")]
#[doc(alias = "xdp_portal_registry")]
/// Wrapper of the DBus interface [`org.freedesktop.host.portal.Registry`](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html).
pub struct Registry;

impl Registry {
    /// Register a D-Bus peer and associate it with an application ID
    pub async fn register(&self, app_id: &str) -> Result<(), Error> {
        let proxy = RegistryProxy::new().await?;

        proxy.register(app_id, HashMap::new()).await
    }
}
