use std::path::Path;
use dicom::object::mem::InMemDicomObject;
use dicom::object::{RootDicomObject, StandardDataDictionary};
use anyhow::{Result, anyhow};


pub type Dicom = RootDicomObject<InMemDicomObject<StandardDataDictionary>>;

#[derive(Debug, Clone)]
pub enum Encoding {
    RAW,
    RLE,
    JPEG
}

#[allow(non_camel_case_types)]
pub enum PhotoInterp {
    RGB,
    Palette(Vec<Vec<u16>>),
    YBR_FULL_422
}

pub struct EncodedImageData {
    pub w: u32,
    pub h: u32,
    pub samples_per_pixel: u32,
    pub bytes_per_sample: u32,
    pub encoding: Encoding,
    pub pixel_data: Vec<u8>,
    pub photo_interp: PhotoInterp
}

pub struct DecodedImageData {
    pub w: u32,
    pub h: u32,
    pub samples_per_pixel: u32,
    pub bytes_per_sample: u32,
    pub pixel_data: Vec<u8>
}

pub fn write_image(image_data: &DecodedImageData, path: &Path) -> Result<()> {

    let DecodedImageData { 
        w, h, pixel_data, bytes_per_sample, samples_per_pixel, .. 
    } = image_data;

    let file = std::fs::File::create(path).unwrap();
    let ref mut file_buf = std::io::BufWriter::new(file);
    
    let mut encoder = png::Encoder::new(file_buf, *w, *h);

    let color = match samples_per_pixel {
        3 => png::ColorType::RGB,
        1 => png::ColorType::Grayscale,
        _ => return Err(anyhow!("Unsupported color type"))
    };

    let bit_depth = match bytes_per_sample {
        1 => png::BitDepth::Eight,
        2 => png::BitDepth::Sixteen,
        _ => return Err(anyhow!("Unsupported bit depth"))
    };


    encoder.set_color(color);
    encoder.set_depth(bit_depth);

    let mut writer = encoder.write_header().unwrap();
    
    writer.write_image_data(pixel_data).unwrap();

    Ok(())
}
