use std::path::PathBuf;
use dicom::object::open_file;
use anyhow::{Result, anyhow};

mod utils;
mod decoding;

use utils::write_image;
use decoding::get_image;

fn main() -> Result<()> {

    let mut args = std::env::args().skip(1);

    let input_path = PathBuf::from(
        args.next().ok_or(anyhow!("Not enough arguments"))?);
    let output_path = PathBuf::from(
        args.next().ok_or(anyhow!("Not enough arguments"))?);

    let dicom = open_file(input_path.as_os_str())?;
    let decoded_image_data = get_image(&dicom)?;
    write_image(&decoded_image_data, output_path.as_path())?;

    Ok(())
}
