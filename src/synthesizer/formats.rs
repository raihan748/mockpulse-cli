//! Realistic format generators for UUIDs, dates, emails, networks, and identifiers.

use rand::Rng;

pub fn uuid_v4() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);

    // Set version to 4 (0100)
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant to RFC 4122 (10xx)
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

pub fn email() -> String {
    let mut rng = rand::thread_rng();
    let id: u32 = rng.gen_range(1000..9999);
    let domains = ["example.com", "mockpulse.dev", "api.internal", "testnet.org"];
    let domain = domains[rng.gen_range(0..domains.len())];
    format!("user_{}@{}", id, domain)
}

pub fn date_time() -> String {
    let mut rng = rand::thread_rng();
    let year = rng.gen_range(2024..2027);
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28);
    let hour = rng.gen_range(0..24);
    let min = rng.gen_range(0..60);
    let sec = rng.gen_range(0..60);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hour, min, sec)
}

pub fn date() -> String {
    let mut rng = rand::thread_rng();
    let year = rng.gen_range(2024..2027);
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28);
    format!("{:04}-{:02}-{:02}", year, month, day)
}

pub fn time() -> String {
    let mut rng = rand::thread_rng();
    let hour = rng.gen_range(0..24);
    let min = rng.gen_range(0..60);
    let sec = rng.gen_range(0..60);
    format!("{:02}:{:02}:{:02}", hour, min, sec)
}

pub fn ipv4() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "{}.{}.{}.{}",
        rng.gen_range(10..220),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(1..254)
    )
}

pub fn ipv6() -> String {
    let mut rng = rand::thread_rng();
    format!(
        "2001:db8:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(0..0xffff),
        rng.gen_range(1..0xffff)
    )
}

pub fn uri() -> String {
    let mut rng = rand::thread_rng();
    let id: u32 = rng.gen_range(100..999);
    format!("https://api.mockpulse.dev/v1/resources/{}", id)
}

pub fn hostname() -> String {
    let mut rng = rand::thread_rng();
    let id: u16 = rng.gen_range(1..99);
    format!("srv-{:02}.mockpulse.internal", id)
}

pub fn base64_data() -> String {
    // Simple valid base64 payload representation
    "bW9ja3B1bHNlLXBheWxvYWQtZGF0YQ==".to_string()
}

pub fn random_string(min_len: Option<usize>, max_len: Option<usize>) -> String {
    let mut rng = rand::thread_rng();
    let min = min_len.unwrap_or(5);
    let max = max_len.unwrap_or(12).max(min);
    let target_len = rng.gen_range(min..=max);

    let charset: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..target_len)
        .map(|_| {
            let idx = rng.gen_range(0..charset.len());
            charset[idx] as char
        })
        .collect()
}
