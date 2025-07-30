use serialport::SerialPort;

use crate::common;

#[derive(Debug, Clone)]
pub struct RawRequestPackage {
    header: u8,
    monitor_id: u8,
    category: u8,
    code0: u8, // Page
    code1: u8, // Function
    length: u8,
    data_control: u8,
    data: Option<Vec<u8>>,
    checksum: u8,
}

impl RawRequestPackage {
    #[must_use]
    fn new(monitor_id: u8, function_code: u8, data: &Option<Vec<u8>>) -> Self {
        let length = data.as_ref().map_or(0, |d| d.len() as u8) + 3;

        let data = vec![function_code]
            .into_iter()
            .chain(data.as_ref().map_or(vec![], |d| d.clone()))
            .collect::<Vec<u8>>();

        let checksum = 0xa6 ^ monitor_id ^ length ^ 0x01 ^ data.iter().fold(0, |acc, &b| acc ^ b);

        Self {
            header: 0xa6,
            monitor_id,
            category: 0x00,
            code0: 0x00,
            code1: 0x00,
            length,
            data_control: 0x01,
            data: Some(data.clone()),
            checksum,
        }
    }
}

/// Enum representing the different request functions for getting data from the monitor
#[derive(Debug, Clone, Copy)]
pub enum GetRequestFunction {
    CommunicationControl,
    PlatformAndVersionLabels,
    PowerState,
    UserInputControl,
    PowerStateAtColdStart,
    CurrentSource,
    VideoParameters,
    ColorTemperature,
    ColorParameters,
    PictureFormat,
    Volume,
    AudioParameters,
    MiscellaneousInfo,
    SerialCode,
}

impl GetRequestFunction {
    /// Returns the command code associated with the request function
    #[must_use]
    fn get_command_code(&self) -> u8 {
        match self {
            GetRequestFunction::CommunicationControl => 0x00,
            GetRequestFunction::PlatformAndVersionLabels => 0xa2,
            GetRequestFunction::PowerState => 0x19,
            GetRequestFunction::UserInputControl => 0x1d,
            GetRequestFunction::PowerStateAtColdStart => 0xa4,
            GetRequestFunction::CurrentSource => 0xad,
            GetRequestFunction::VideoParameters => 0x33,
            GetRequestFunction::ColorTemperature => 0x35,
            GetRequestFunction::ColorParameters => 0x37,
            GetRequestFunction::PictureFormat => 0x3b,
            GetRequestFunction::Volume => 0x45,
            GetRequestFunction::AudioParameters => 0x43,
            GetRequestFunction::MiscellaneousInfo => 0x0f,
            GetRequestFunction::SerialCode => 0x15,
        }
    }
}

/// Enum representing the different request functions for setting data on the monitor
#[derive(Debug, Clone, Copy)]
pub enum SetRequestFunction {
    CommunicationControl,
    PowerState(common::PowerState),
    UserInputControl,
    PowerStateAtColdStart,
    InputSource,
    VideoParameters,
    ColorTemperature,
    ColorParameters,
    PictureFormat,
    Volume,
    VolumeLimits,
    AudioParameters,
    AutoAdjust,
}

impl SetRequestFunction {
    /// Returns the command code associated with the request function
    #[must_use]
    fn get_command_code(&self) -> u8 {
        match self {
            SetRequestFunction::CommunicationControl => 0x00,
            SetRequestFunction::PowerState(_) => 0x18,
            SetRequestFunction::UserInputControl => 0x1c,
            SetRequestFunction::PowerStateAtColdStart => 0xa3,
            SetRequestFunction::InputSource => 0xac,
            SetRequestFunction::VideoParameters => 0x32,
            SetRequestFunction::ColorTemperature => 0x34,
            SetRequestFunction::ColorParameters => 0x36,
            SetRequestFunction::PictureFormat => 0x3a,
            SetRequestFunction::Volume => 0x44,
            SetRequestFunction::VolumeLimits => 0xb8,
            SetRequestFunction::AudioParameters => 0x42,
            SetRequestFunction::AutoAdjust => 0x70,
        }
    }

    #[must_use]
    fn get_payload_data(&self) -> Option<Vec<u8>> {
        match self {
            SetRequestFunction::PowerState(state) => Some(vec![match state {
                common::PowerState::Off => 0x01,
                common::PowerState::On => 0x02,
            }]),
            _ => None,
        }
    }

    #[must_use]
    pub fn from_cli(command: &str, value: &str) -> Option<Self> {
        match command {
            "power" => match value {
                "on" => Some(SetRequestFunction::PowerState(common::PowerState::On)),
                "off" => Some(SetRequestFunction::PowerState(common::PowerState::Off)),
                _ => None,
            },
            "input" => Some(SetRequestFunction::InputSource), // Placeholder for input source handling
            _ => None,
        }
    }
}

pub struct GetRequest {
    pub monitor_id: u8,
    pub function: GetRequestFunction,
}

pub fn set<T: SerialPort>(
    monitor_id: u8,
    function: SetRequestFunction,
    port: &mut T,
) -> Result< {
    let command_code = function.get_command_code();
    let data = function.get_payload_data();

    let request = RawRequestPackage::new(monitor_id, command_code, &data);
    port.write_all(&Vec::<u8>::from(request))
        .expect("Failed to write to port");
}

impl From<RawRequestPackage> for Vec<u8> {
    fn from(package: RawRequestPackage) -> Self {
        let mut data = vec![
            package.header,
            package.monitor_id,
            package.category,
            package.code0,
            package.code1,
            package.length,
            package.data_control,
        ];
        if let Some(ref d) = package.data {
            data.extend_from_slice(d);
        }
        data.push(package.checksum);
        data
    }
}

pub enum RawResponseStatus<T> {
    Acknowledged(T),
    NotAcknowledged,
    NotAvailable,
}

pub type RawSetReponseStatus = RawResponseStatus<()>;
pub type RawGetResponseStatus<T> = RawResponseStatus<T>;

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the creation of a RawRequestPackage with simple data.
    #[test]
    fn test_raw_request_package() {
        let package = RawRequestPackage::new(1, 0x19, &Some(vec![0x01]));
        assert_eq!(package.header, 0xa6);
        assert_eq!(package.monitor_id, 1);
        assert_eq!(package.category, 0x00);
        assert_eq!(package.code0, 0x00);
        assert_eq!(package.code1, 0x00);
        assert_eq!(package.length, 4);
        assert_eq!(package.data_control, 0x01);
        assert_eq!(package.data.unwrap(), vec![0x19, 0x01]);
        assert_eq!(package.checksum, 0xa6 ^ 1 ^ 4 ^ 0x01 ^ 0x19 ^ 0x01);
    }

    /// Tests the creation of a RawRequestPackage with no data.
    #[test]
    fn test_raw_request_package_no_data() {
        let package = RawRequestPackage::new(1, 0x19, &None);
        assert_eq!(package.header, 0xa6);
        assert_eq!(package.monitor_id, 1);
        assert_eq!(package.category, 0x00);
        assert_eq!(package.code0, 0x00);
        assert_eq!(package.code1, 0x00);
        assert_eq!(package.length, 3);
        assert_eq!(package.data_control, 0x01);
        assert_eq!(package.data, Some(vec![0x19]));
        assert_eq!(package.checksum, 0xa6 ^ 1 ^ 3 ^ 1 ^ 0x19);
    }

    #[test]
    fn test_set_power_state_off() {
        let package = set(1, SetRequestFunction::PowerState(common::PowerState::Off));
        assert_eq!(package.header, 0xa6);
        assert_eq!(package.monitor_id, 1);
        assert_eq!(package.category, 0x00);
        assert_eq!(package.code0, 0x00);
        assert_eq!(package.code1, 0x00);
        assert_eq!(package.length, 0x04);
        assert_eq!(package.data_control, 0x01);
        assert_eq!(package.data.unwrap(), vec![0x18, 0x01]);
        assert_eq!(package.checksum, 0xbb);
    }

    #[test]
    fn test_set_power_state_on() {
        let package = set(1, SetRequestFunction::PowerState(common::PowerState::On));
        assert_eq!(package.header, 0xa6);
        assert_eq!(package.monitor_id, 1);
        assert_eq!(package.category, 0x00);
        assert_eq!(package.code0, 0x00);
        assert_eq!(package.code1, 0x00);
        assert_eq!(package.length, 0x04);
        assert_eq!(package.data_control, 0x01);
        assert_eq!(package.data.unwrap(), vec![0x18, 0x02]);
        assert_eq!(package.checksum, 0xb8);
    }
}
