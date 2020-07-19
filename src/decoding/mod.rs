use std::convert::TryInto;
use jpeg2000::decode::{Codec, DecodeConfig};
use anyhow::{Result, anyhow};
use crate::utils::{Dicom, EncodedImageData, DecodedImageData, Encoding, PhotoInterp};

mod dicom_parsing;
use dicom_parsing::get_encoded_image_data;


pub fn get_image(dicom: Dicom) -> Result<DecodedImageData> {
    let encoded_image_data = get_encoded_image_data(&dicom)?;
    let decoded_image_data = decode_image(encoded_image_data)?;
    Ok(decoded_image_data)
}


fn decode_image(encoded_image_data: EncodedImageData) -> Result<DecodedImageData> {

    let decoded_bytes_1 = match encoded_image_data.encoding {
        Encoding::RAW => encoded_image_data.pixel_data.clone(),
        Encoding::RLE => decode_RLE(&encoded_image_data)?,
        Encoding::JPEG => decode_JPEG(&encoded_image_data)?,
        Encoding::JPEG2000 => decode_JPEG2000(&encoded_image_data)?
    };

    let (decoded_bytes_2, new_samples_per_pixel) = match encoded_image_data.photo_interp {
        PhotoInterp::Palette(_) => {
            let decoded_bytes_2 = map_to_palette(&encoded_image_data, decoded_bytes_1)?;
            (decoded_bytes_2, 3)
        },
        PhotoInterp::RGB => (decoded_bytes_1, encoded_image_data.samples_per_pixel),
        PhotoInterp::YBR_FULL_422 => (decoded_bytes_1, encoded_image_data.samples_per_pixel), // TESTING !
        PhotoInterp::MONOCHROME2 => (decoded_bytes_1, encoded_image_data.samples_per_pixel)
    };

    Ok(DecodedImageData {
        w: encoded_image_data.w,
        h: encoded_image_data.h,
        samples_per_pixel: new_samples_per_pixel,
        bytes_per_sample: encoded_image_data.bytes_per_sample,
        pixel_data: decoded_bytes_2
    })
}

#[allow(non_snake_case)]
fn decode_JPEG(encoded_image_data: &EncodedImageData) -> Result<Vec<u8>> {

    let mut decoder = jpeg_decoder::Decoder::new(encoded_image_data.pixel_data.as_slice());
    let decoded_pixel_data = decoder.decode()?;
    Ok(decoded_pixel_data)
}

#[allow(non_snake_case)]
fn decode_JPEG2000(encoded_image_data: &EncodedImageData) -> Result<Vec<u8>> {

    let EncodedImageData { samples_per_pixel, bytes_per_sample, .. } = encoded_image_data;

    let dynamic_image = jpeg2000::decode::from_memory(
        encoded_image_data.pixel_data.as_slice(),
        Codec::JP2,
        DecodeConfig {
            default_colorspace: None,
            discard_level: 0,
        },
        None,
    )?;

    let pixel_data: Vec<u8> = match (samples_per_pixel, bytes_per_sample) {
        (1, 1) => dynamic_image.to_luma().into_vec(),
        (3, 1) => dynamic_image.to_rgb().into_vec(),
        (4, 1) => dynamic_image.to_rgba().into_vec(),
        _ => return Err(anyhow!(
            "JPEG2000 output {} samples per pixel, {}  bytes per sample is unsupported",
            samples_per_pixel, bytes_per_sample
        ))
    };

    Ok(pixel_data)
}

fn map_to_palette(encoded_image_data: &EncodedImageData, decoded_bytes: Vec<u8>) -> Result<Vec<u8>> {

    let palettes = match &encoded_image_data.photo_interp {
        PhotoInterp::Palette(palettes) => palettes,
        _ => panic!()
    };

    let EncodedImageData { samples_per_pixel, bytes_per_sample, .. } = encoded_image_data;

    if *samples_per_pixel != 1 { 
        return Err(anyhow!(
            "Image requires palette mapping but has {} > 1 samples per pixel",
            samples_per_pixel
        ))
    }

    if *bytes_per_sample != 1 && *bytes_per_sample != 2 {
        return Err(anyhow!(
            "Unsupported bit depth: {} (1 and 2 supported)",
            *bytes_per_sample
        ))  
    }

    let d = *bytes_per_sample as usize;
    let mapped_bytes: Vec<u8> = decoded_bytes
        .chunks_exact(d)
        .map(|v| match d {
            1 => u8::from_be_bytes([v[0]]) as usize,
            2 => u16::from_be_bytes([v[0], v[1]]) as usize,
            _ => panic!()
        })
        .flat_map(|index| {

            let mut px_bytes: Vec<u8> = Vec::new();
            for i in 0..3 {
                let [b1, b2] = palettes[i][index].to_be_bytes();
                px_bytes.push(b1);
                px_bytes.push(b2);
            }
            px_bytes

        })
        .collect();

    Ok(mapped_bytes)
}

#[allow(non_snake_case)]
fn decode_RLE(encoded_data: &EncodedImageData) -> Result<Vec<u8>> {

    let EncodedImageData { pixel_data: encoded_pixel_data, .. } = encoded_data;

    /*
        Decoding segments
    */

    let mut offsets = decode_header(&encoded_pixel_data)?;
    offsets.push(encoded_pixel_data.len());
    let nb_segments = offsets.len();

    let mut decoded_segments = Vec::new();
    for i in 0..nb_segments-1 {
        let o1 = offsets[i];
        let o2 = offsets[i+1];
        let segment_data = &encoded_pixel_data[o1..o2];
        decoded_segments.push(decode_segment(segment_data)?);
    }

    /*
        Interlacing segments
    */

    let seg_len = decoded_segments[0].len();
    let mut decoded_pixel_data = Vec::new();
    for i in 0..seg_len {
        for segment_data in decoded_segments.iter() {
            decoded_pixel_data.push(segment_data[i]);
        }
    }

    Ok(decoded_pixel_data)
}


fn decode_header(pixel_data: &Vec<u8>) -> Result<Vec<usize>> {

    if pixel_data.len() < 64 {
        return Err(anyhow!("RLE: not enough bytes for header"))
    }

    let nb_segments = u32::from_le_bytes((&pixel_data[..4]).try_into()?);

    if nb_segments == 0 {
        return Err(anyhow!("RLE: header has zero segments"))
    }

    let mut offsets = Vec::new();
    let n = std::cmp::min(nb_segments, 16);
    for i in 1..n+1 {
        let x = (i * 4) as usize;
        let o = u32::from_le_bytes((&pixel_data[x..x+4]).try_into()?);
        offsets.push(o as usize);
    }

    Ok(offsets)
}

fn decode_segment(segment_data: &[u8]) -> Result<Vec<u8>> {

    #[allow(non_snake_case)]
    let N = segment_data.len();

    let mut decoded_segment = Vec::<u8>::new();
    let mut i = 0;

    loop {

        if i >= N || i + 1 >= N {
            break;
        }

        let header_val = segment_data[i] + 1;

        i += 1;

        if header_val > 129 {

            let n = (3 + (255 - header_val)) as usize;
            let rep_byte = segment_data[i];
            decoded_segment.append(&mut vec![rep_byte; n]);
            i += 1;

        } else if header_val < 129 {

            let n = header_val as usize;
            if i+n > N { break; }
            decoded_segment.append(&mut segment_data[i..i+n].to_vec());
            i += n;

        }
    }

    Ok(decoded_segment)
}
