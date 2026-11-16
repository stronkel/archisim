//! What changes between the desktop and the web (`docs/IPAD.md`).
//!
//! In the browser there are no threads of our own: what the desktop does
//! aside runs at once, and the window waits for it, except what goes to
//! the browser's workers (`web::render_aside`, T5). The services of the
//! surroundings are reached through the NAS's bridge (`bridged`).

/// Whether this is the version for the browser.
pub const WEB: bool = cfg!(target_arch = "wasm32");

/// Runs `work` aside: on another thread on the desktop, at once on the web.
/// Whatever it hands back through a channel or a shared slot is there when
/// the window looks for it, as it would be after the thread ended.
pub fn spawn(work: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(work);
    }
    #[cfg(target_arch = "wasm32")]
    {
        work();
    }
}

/// How many pieces a long job can be split into at the same time.
#[allow(dead_code)]
pub fn parallelism() -> usize {
    if WEB {
        1
    } else {
        std::thread::available_parallelism().map_or(4, |n| n.get())
    }
}

/// The servers of the surroundings (`archisim_library::download` and
/// `geocode`) that the NAS lets the web version reach through itself
/// (`scripts/nas/Caddyfile`, `/proxy/<server>/…`): from a page, the
/// browser does not let most of them answer.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub const BRIDGED_HOSTS: &[&str] = &[
    "ovc.catastro.meh.es",
    "overpass-api.de",
    "overpass.kumi.systems",
    "overpass.private.coffee",
    "api.openstreetmap.org",
    "nominatim.openstreetmap.org",
    "www.cartociudad.es",
    "www.ign.es",
    "xyz-mdt.idee.es",
    "wcs-mds.idee.es",
];

/// The address through the NAS's bridge (`proxy/<server>/<rest>`, beside
/// the page) when there is one and the server is one of
/// [`BRIDGED_HOSTS`]; the address as it is if not.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn bridged(url: &str, bridge: bool) -> String {
    if !bridge {
        return url.to_string();
    }
    let Some(rest) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    else {
        return url.to_string();
    };
    let host = rest.split(['/', '?']).next().unwrap_or(rest);
    if BRIDGED_HOSTS.contains(&host) {
        format!("proxy/{rest}")
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_takes_the_services_of_the_surroundings() {
        assert_eq!(
            bridged("http://ovc.catastro.meh.es/INSPIRE/wfsBU.aspx?a=1", true),
            "proxy/ovc.catastro.meh.es/INSPIRE/wfsBU.aspx?a=1"
        );
        assert_eq!(
            bridged("https://www.ign.es/wms-inspire/pnoa-ma?x", true),
            "proxy/www.ign.es/wms-inspire/pnoa-ma?x"
        );
        assert_eq!(
            bridged("https://example.com/a", true),
            "https://example.com/a"
        );
        assert_eq!(
            bridged("https://overpass-api.de/api/interpreter", false),
            "https://overpass-api.de/api/interpreter"
        );
    }

    /// Every server the program asks is bridged, and the NAS bridges it.
    #[test]
    fn the_nas_bridges_every_service() {
        let sources = [
            include_str!("../../archisim-library/src/download.rs"),
            include_str!("../../archisim-library/src/geocode.rs"),
        ];
        let caddy = include_str!("../../../scripts/nas/Caddyfile");
        for text in sources {
            for (i, _) in text.match_indices("https://") {
                let rest = &text[i + 8..];
                let host: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-')
                    .collect();
                if host.is_empty() || host.contains("github") {
                    continue;
                }
                assert!(BRIDGED_HOSTS.contains(&host.as_str()), "{host} sin puente");
            }
        }
        for host in BRIDGED_HOSTS {
            assert!(
                caddy.contains(&format!("import puente {host}")),
                "el Caddyfile no lleva el puente a {host}"
            );
        }
    }
}
