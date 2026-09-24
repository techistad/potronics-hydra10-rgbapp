use hidapi::{DeviceInfo, HidApi, HidDevice};

const VENDOR_ID: u16 = 0x258A;
const PREFERRED_PRODUCT: u16 = 0x010C;
const USAGE_PAGE: u16 = 0xFF00;
const REPORT_LEN: usize = 520;

pub type Rgb = [u8; 3];

pub fn build_packet(leds: &[Rgb]) -> Vec<u8> {
    let mut payload = vec![0u8; 96 * 3];
    for (index, color) in leds.iter().take(96).enumerate() {
        let base = index * 3;
        payload[base] = color[0];
        payload[base + 1] = color[1];
        payload[base + 2] = color[2];
    }
    let mut packet = vec![0x06, 0x08, 0x00, 0x00, 0x01, 0x00, 0x7A, 0x01];
    packet.extend(payload);
    packet.resize(REPORT_LEN, 0);
    packet
}

fn path_text(info: &DeviceInfo) -> String {
    info.path().to_string_lossy().to_string()
}

fn is_color_endpoint(info: &DeviceInfo) -> bool {
    info.vendor_id() == VENDOR_ID
        && info.usage_page() == USAGE_PAGE
        && info.usage() == 1
        && path_text(info).to_ascii_lowercase().contains("col06")
}

pub fn wireless_note(api: &HidApi) -> String {
    let mut bluetooth = String::new();
    let mut dongle = String::new();
    for info in api.device_list() {
        let product = info.product_string().unwrap_or("");
        if !product.to_ascii_lowercase().contains("hydra") {
            continue;
        }
        let path = path_text(info).to_ascii_lowercase();
        if path.contains("00001812") || path.contains("bthenum") || path.contains("bthle") {
            bluetooth = product.to_string();
        } else if info.vendor_id() == 0x3554 {
            dongle = product.to_string();
        }
    }
    if !dongle.is_empty() {
        return format!(
            "{dongle} is on the 2.4 GHz dongle. Keys show here because Windows sees them. That dongle only stores the built-in effect colors, so these live effects cannot drive the keys. Plug in the USB-C cable."
        );
    }
    if !bluetooth.is_empty() {
        return format!(
            "{bluetooth} is connected over Bluetooth, so typing works. That link has no color channel. Plug in the USB-C cable to change the lights."
        );
    }
    "No lighting channel found. Plug the keyboard in with the USB-C cable.".into()
}

pub struct OpenedKeyboard {
    pub device: HidDevice,
    pub product: String,
    pub product_id: u16,
}

pub fn open_keyboard(api: &mut HidApi) -> Result<OpenedKeyboard, String> {
    api.refresh_devices().map_err(|err| err.to_string())?;
    let mut matches: Vec<&DeviceInfo> = api.device_list().filter(|info| is_color_endpoint(info)).collect();
    matches.sort_by_key(|info| if info.product_id() == PREFERRED_PRODUCT { 0 } else { 1 });
    let Some(info) = matches.first() else {
        return Err(wireless_note(api));
    };
    let device = info.open_device(api).map_err(|err| format!("Could not open the keyboard: {err}"))?;
    Ok(OpenedKeyboard {
        device,
        product: info.product_string().unwrap_or("Hydra 10").to_string(),
        product_id: info.product_id(),
    })
}

pub fn write_leds(device: &HidDevice, leds: &[Rgb]) -> Result<(), String> {
    let packet = build_packet(leds);
    device
        .send_feature_report(&packet)
        .map(|_| ())
        .map_err(|err| err.to_string())
}
