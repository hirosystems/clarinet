use failure::Error;
use json;
use std::fs;

#[derive(Debug, Clone)]
pub struct Control {
    pub control_port: u16,
    pub shell_port: u16,
    pub stdin_port: u16,
    pub hb_port: u16,
    pub iopub_port: u16,
    pub transport: String,
    pub signature_scheme: String,
    pub ip: String,
    pub key: String,
}

macro_rules! parse_to_var {
    ($control_json:expr, $name:ident, $convert:ident) => {
        let $name = $control_json[stringify!($name)]
            .$convert()
            .ok_or_else(|| format_err!("Missing JSON field {}", stringify!($name)))?;
    };
}

impl Control {
    pub fn parse_file(file_name: &str) -> Result<Control, Error> {
        let control_file_contents = fs::read_to_string(file_name)?;
        let control_json = json::parse(&control_file_contents)?;
        parse_to_var!(control_json, control_port, as_u16);
        parse_to_var!(control_json, shell_port, as_u16);
        parse_to_var!(control_json, stdin_port, as_u16);
        parse_to_var!(control_json, hb_port, as_u16);
        parse_to_var!(control_json, iopub_port, as_u16);
        parse_to_var!(control_json, transport, as_str);
        parse_to_var!(control_json, signature_scheme, as_str);
        parse_to_var!(control_json, ip, as_str);
        parse_to_var!(control_json, key, as_str);
        Ok(Control {
            control_port,
            shell_port,
            stdin_port,
            hb_port,
            iopub_port,
            transport: transport.to_owned(),
            signature_scheme: signature_scheme.to_owned(),
            key: key.to_owned(),
            ip: ip.to_owned(),
        })
    }
}
