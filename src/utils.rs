use std::convert::TryInto;
use std::path::Path;
use dicom::object::mem::InMemDicomObject;
use dicom::object::{RootDicomObject, StandardDataDictionary};
use anyhow::{Result, anyhow};


pub type Dicom = RootDicomObject<InMemDicomObject<StandardDataDictionary>>;

#[derive(Debug, Clone)]
pub enum Encoding {
    RAW,
    RLE,
    JPEG,
    JPEG2000
}

pub type Palettes = Vec<Vec<u16>>;

#[derive(Clone)]
pub struct Format {
    pub w: u32,
    pub h: u32,
    pub channels: u32,
    pub channel_depth: u32
}

pub struct EncodedImage {
    pub target_format: Format,
    pub encoding: Encoding,
    pub palettes: Option<Palettes>,
    pub bytes: Vec<u8>
}

pub struct RawImage {
    pub format: Format,
    pub bytes: Vec<u8>
}

pub fn write_image(image: &RawImage, path: &Path) -> Result<()> {

    let RawImage { format, bytes } = image;

    let file = std::fs::File::create(path).unwrap();
    let ref mut file_buf = std::io::BufWriter::new(file);
    
    let mut encoder = png::Encoder::new(file_buf, format.w, format.h);

    let color = match format.channels {
        3 => png::ColorType::RGB,
        1 => png::ColorType::Grayscale,
        _ => return Err(anyhow!("Unsupported color type"))
    };

    let bit_depth = match format.channel_depth {
        1 => png::BitDepth::Eight,
        2 => png::BitDepth::Sixteen,
        _ => return Err(anyhow!("Unsupported bit depth"))
    };


    encoder.set_color(color);
    encoder.set_depth(bit_depth);

    let mut writer = encoder.write_header().unwrap();
    
    writer.write_image_data(bytes).unwrap();

    Ok(())
}

#[allow(non_snake_case)]
pub fn convert_to_BGRA8888(image: &RawImage) -> Result<RawImage> {

    let RawImage { format, bytes } = image;
    let Format { channels, channel_depth, .. } = format;

    let u8_bytes: Vec<u8> = match channel_depth {

        1 => bytes.clone(),

        2 => {

            bytes
                .chunks_exact(2)
                .map(|chunk| -> u8 {
                    let val: u16 = u16::from_le_bytes(chunk.try_into().unwrap())
                        .try_into().unwrap();
                    (val >> 8).try_into().unwrap()
                })
                .collect()

        },

        _ => return Err(anyhow!(
            "RGBA conversion: unsupported format: {} channels of depth {} bytes",
            *channels, *channel_depth
        ))
    };

    let rgba_bytes: Vec<u8> = match channels  {

        3 => u8_bytes
                .chunks_exact(3)
                .flat_map(|chunk| {
                    let [r, g, b]: [u8; 3] = chunk.try_into().unwrap();
                    vec![b, g, r, 255]
                })
                .collect(),

        1 => u8_bytes
                .iter()
                .flat_map(|v| vec![*v, *v, *v, 255])
                .collect(),  
                
        _ => return Err(anyhow!(
            "RGBA conversion: unsupported format: {} channels of depth {} bytes",
            *channels, *channel_depth
        ))
    };

    let mut new_format = format.clone();
    new_format.channels = 3;
    new_format.channel_depth = 1;

    Ok(RawImage { bytes: rgba_bytes, format: new_format } )
}
