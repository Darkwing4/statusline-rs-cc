use std::env;
use std::fs;
use std::path::Path;

const TIMEZONE_FILE: &str = "/etc/timezone";
const LOCALTIME_LINK: &str = "/etc/localtime";
const ZONEINFO_MARKER: &str = "zoneinfo/";

pub(super) fn system_location() -> Option<String> {
    city_from_timezone(&system_timezone()?)
}

fn system_timezone() -> Option<String> {
    if let Some(configured) = env::var_os("TZ").and_then(|value| value.into_string().ok()) {
        let trimmed = configured.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Ok(body) = fs::read_to_string(TIMEZONE_FILE) {
        let trimmed = body.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let target = fs::read_link(LOCALTIME_LINK).ok()?;

    zone_from_link(&target)
}

fn zone_from_link(target: &Path) -> Option<String> {
    let path = target.to_str()?;
    let start = path.find(ZONEINFO_MARKER)? + ZONEINFO_MARKER.len();

    Some(path[start..].to_string())
}

fn city_from_timezone(timezone: &str) -> Option<String> {
    let (_, city) = timezone.trim().rsplit_once('/')?;
    let city = city.trim();

    if city.is_empty() {
        return None;
    }

    Some(city.replace('_', " "))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{city_from_timezone, zone_from_link};

    #[test]
    fn takes_the_city_from_an_iana_timezone() {
        assert_eq!(
            city_from_timezone("Europe/Moscow").as_deref(),
            Some("Moscow")
        );
        assert_eq!(
            city_from_timezone("America/New_York").as_deref(),
            Some("New York")
        );
        assert_eq!(
            city_from_timezone("America/Argentina/Buenos_Aires").as_deref(),
            Some("Buenos Aires")
        );
    }

    #[test]
    fn rejects_timezones_that_name_no_city() {
        assert_eq!(city_from_timezone("UTC"), None);
        assert_eq!(city_from_timezone(""), None);
        assert_eq!(city_from_timezone("Europe/"), None);
    }

    #[test]
    fn reads_the_zone_out_of_a_localtime_link() {
        assert_eq!(
            zone_from_link(Path::new("/usr/share/zoneinfo/Europe/Moscow")).as_deref(),
            Some("Europe/Moscow")
        );
        assert_eq!(
            zone_from_link(Path::new("../usr/share/zoneinfo/Asia/Tbilisi")).as_deref(),
            Some("Asia/Tbilisi")
        );
        assert_eq!(zone_from_link(Path::new("/etc/localtime")), None);
    }
}
