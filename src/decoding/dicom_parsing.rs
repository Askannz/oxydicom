use dicom::object::Tag;
use dicom::core::value::{Value, PrimitiveValue};
use anyhow::{Context, Result, bail, ensure};

use crate::utils::{EncodedImage, Encoding, Dicom, Format, Palettes};


pub fn get_encoded_image_data(dicom: &Dicom) -> Result<EncodedImage> {

    /*
        Dimensions
    */

    let w = dicom.element(Tag(0x0028, 0x0011))?.to_int()?;
    let h = dicom.element(Tag(0x0028, 0x0010))?.to_int()?;
    let bits_per_sample: u32 = dicom.element(Tag(0x0028, 0x0100))?.to_int()?;
    let samples_per_pixel = dicom.element(Tag(0x0028, 0x0002))?.to_int()?;

    ensure!(bits_per_sample % 8 == 0, "Bad bits_per_sample: {}", bits_per_sample);
    let bytes_per_sample = bits_per_sample / 8;

    /*
        Encoding
    */

    let uid = dicom.meta().transfer_syntax.as_str();
    let uid = uid.trim_end_matches(char::from(0)); // Get rid of null terminators

    let encoding = match uid {
        "1.2.840.10008.1.2.1"       => Encoding::RAW,
        "1.2.840.10008.1.2.4.50"    => Encoding::JPEG,
        "1.2.840.10008.1.2.4.90"    => Encoding::JPEG2000,
        "1.2.840.10008.1.2.5"       => Encoding::RLE,
        val => bail!("Unhandled transfer syntax: {}", val)
    };

    /*
        Pixel data
    */

    let pixel_bytes = match dicom.element(Tag(0x7FE0, 0x0010))?.value() {

        Value::Primitive(PrimitiveValue::U8(pixel_bytes)) => pixel_bytes.as_slice(),
        Value::Primitive(PrimitiveValue::U16(_)) => unimplemented!(),
        Value::PixelSequence { fragments, .. } => {
            let frag = fragments.as_slice();
            &frag[0]
        }

        val => bail!("Unexpected pixel data type: {:?}", val)
    };

    let pixel_bytes = pixel_bytes.to_vec();

    /*
        Photometric interpretation
    */

    let photo_interp_str = dicom.element(Tag(0x0028, 0x0004))?.to_str()?;
    let mut photo_interp_str = photo_interp_str.trim_end_matches(char::from(0)).to_owned(); // Get rid of null terminators
    photo_interp_str.retain(|c| !c.is_whitespace()); // Get rid of whitespaces

    let palettes = match photo_interp_str.as_ref() {
        "PALETTECOLOR" => Some(get_palettes(dicom)?),
        _ => None
    };

    Ok(EncodedImage { 

        target_format: Format {
            w, h,
            channels: samples_per_pixel,
            channel_depth: bytes_per_sample
        },
        encoding,
        palettes,
        bytes: pixel_bytes
    })
}

fn get_palettes(dicom: &Dicom) -> Result<Palettes> {

    const TAGS_MAP: [Tag; 3] = [
        Tag(0x0028, 0x1201), // RED
        Tag(0x0028, 0x1202), // GREEN
        Tag(0x0028, 0x1203)  // BLUE
    ];

    TAGS_MAP.iter()
        .copied()
        .map(|tag| {
            let elem = dicom.element(tag)
                .context("Missing palette data element")?;
            let palette = elem.to_multi_int()
                .context("Unexpected palette data type")?;
            Ok(palette)
        })
        .collect()
} 
