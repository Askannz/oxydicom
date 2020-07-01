use dicom::object::Tag;
use dicom::core::value::{Value, PrimitiveValue};
use anyhow::{Result, anyhow};

use crate::utils::{EncodedImageData, Encoding, PhotoInterp, Dicom};


pub fn get_encoded_image_data(dicom: &Dicom) -> Result<EncodedImageData> {

    let get_int_value = |tag| match dicom.element(tag)?.value() {

        Value::Primitive(primitive_val) => match primitive_val {
            PrimitiveValue::U16(val) => Ok(val[0] as u32),
            _ => return Err(anyhow!("Not U16 data"))
        },

        val => return Err(anyhow!("Not a primitive: {:?}", val))
    };

    /*
        Dimensions
    */

    let w = get_int_value(Tag(0x0028, 0x0011))?;
    let h = get_int_value(Tag(0x0028, 0x0010))?;
    let bits_per_sample = get_int_value(Tag(0x0028, 0x0100))?;
    let samples_per_pixel = get_int_value(Tag(0x0028, 0x0002))?;

    assert!(bits_per_sample % 8 == 0);
    let bytes_per_sample = bits_per_sample / 8;

    /*
        Encoding
    */

    let uid = dicom.meta().transfer_syntax.as_str();
    let uid = uid.trim_end_matches(char::from(0)); // Get rid of null terminators

    let encoding = match uid {
        "1.2.840.10008.1.2.1"       => Encoding::RAW,
        "1.2.840.10008.1.2.4.50"    => Encoding::JPEG,
        "1.2.840.10008.1.2.4.90"    => unimplemented!(),
        "1.2.840.10008.1.2.5"       => Encoding::RLE,
        val => return Err(anyhow!("Unhandled transfer syntax: {}", val))
    };

    /*
        Pixel data
    */

    let pixel_data = match dicom.element(Tag(0x7FE0, 0x0010))?.value() {

        Value::Primitive(primitive_val) => match primitive_val {
            PrimitiveValue::U8(pixel_data) => pixel_data.as_slice(),
            PrimitiveValue::U16(_) => unimplemented!(),
            _ => return Err(anyhow!("Unexpected pixel data type"))
        },

        Value::PixelSequence { fragments, .. } => {
            let frag = fragments.as_slice();
            &frag[0]
        }

        val => return Err(anyhow!("Unhandled value type: {:?}", val))
    };

    let pixel_data = pixel_data.to_vec();

    /*
        Photometric interpretation
    */

    let photo_interp_str = dicom.element(Tag(0x0028, 0x0004))?.to_str()?;
    let mut photo_interp_str = photo_interp_str.trim_end_matches(char::from(0)).to_owned(); // Get rid of null terminators
    photo_interp_str.retain(|c| !c.is_whitespace()); // Get rid of whitespaces

    let photo_interp = match photo_interp_str.as_ref() {
        "RGB" => PhotoInterp::RGB,
        "PALETTECOLOR" => PhotoInterp::Palette(get_palettes(dicom)?),
        "YBR_FULL_422" => PhotoInterp::YBR_FULL_422,
        val => return Err(anyhow!("Unhandled photometric interpretation: {}", val))
    };

    Ok(EncodedImageData { 
        w, h, samples_per_pixel, bytes_per_sample, 
        encoding, pixel_data, photo_interp
    })
}

fn get_palettes(dicom: &Dicom) -> Result<Vec<Vec<u16>>> {

    const TAGS_MAP: [Tag; 3] = [
        Tag(0x0028, 0x1201), // RED
        Tag(0x0028, 0x1202), // GREEN
        Tag(0x0028, 0x1203)  // BLUE
    ];

    let mut palettes: Vec<Vec<u16>> = Vec::new();

    for i in 0..3 {

        let palette = match dicom.element(TAGS_MAP[i])?.value() {

            Value::Primitive(primitive_val) => match primitive_val {
                PrimitiveValue::U16(palette_data) => palette_data.as_slice(),
                _ => return Err(anyhow!("Unexpected palette data type"))
            },
    
            val => return Err(anyhow!("Unhandled value type: {:?}", val))
        };

        let palette = palette.to_vec();

        palettes.push(palette);
    }

    Ok(palettes)
} 
