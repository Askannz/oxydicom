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

#[allow(non_camel_case_types)]
pub enum PhotoInterp {
    RGB,
    Palette(Vec<Vec<u16>>),
    YBR_FULL_422,
    MONOCHROME2
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

#[allow(non_snake_case)]
pub fn convert_to_BGRA(image_data: &DecodedImageData) -> Result<DecodedImageData> {

    let DecodedImageData { 
        pixel_data, bytes_per_sample, samples_per_pixel, .. 
    } = image_data;

    let rgba_data: Vec<u8> = match (*bytes_per_sample, *samples_per_pixel)  {

        (1, 3) => pixel_data
                .chunks_exact(3)
                .flat_map(|chunk| {
                    let [r, g, b] = match chunk {
                        [r, g, b] => [*r, *g, *b],
                        _ => panic!()
                    };
                    vec![b, g, r, 255]
                })
                .collect(),

        (1, 1) => pixel_data
                .iter()
                .flat_map(|v| vec![*v, *v, *v, 255])
                .collect(),  
                
        _ => return Err(anyhow!(
            "Unsupported image type: {} bytes per sample, {} samples per pixel",
            *bytes_per_sample, *samples_per_pixel
        ))
    };

    Ok(DecodedImageData {
        w: image_data.w,
        h: image_data.h,
        samples_per_pixel: *samples_per_pixel,
        bytes_per_sample: *bytes_per_sample,
        pixel_data: rgba_data
    })
}
