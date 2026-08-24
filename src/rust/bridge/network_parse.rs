fn ipv4_octets(s: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut octets = [0u8; 4];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || part.len() > 3 || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        octets[index] = part.parse::<u8>().ok()?;
    }
    Some(octets)
}

pub(super) fn is_valid_ipv4(s: &str) -> bool {
    ipv4_octets(s).is_some()
}

pub(super) fn is_tailscale_ipv4(s: &str) -> bool {
    matches!(ipv4_octets(s), Some([100, second, _, _]) if (64..=127).contains(&second))
}

pub(super) fn parse_first_ipv4_line(output: &str) -> Option<String> {
    output
        .lines()
        .map(str::trim)
        .find(|line| is_valid_ipv4(line))
        .map(ToOwned::to_owned)
}

pub(super) fn parse_first_tailscale_ipv4_from_ifconfig(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let value = line.trim().strip_prefix("inet ")?;
        let ip = value.split_whitespace().next()?;
        is_tailscale_ipv4(ip).then(|| ip.to_string())
    })
}
