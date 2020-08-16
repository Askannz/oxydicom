use std::convert::TryInto;
use jpeg2000::decode::{Codec, DecodeConfig};
use anyhow::{Result, anyhow};
use crate::utils::{Dicom, EncodedImage, RawImage, Encoding, Format, Palettes};

mod dicom_parsing;
use dicom_parsing::get_encoded_image_data;


pub fn get_image(dicom: &Dicom) -> Result<RawImage> {
    let encoded_image = get_encoded_image_data(dicom)?;
    let image = decode_image(&encoded_image)?;
    Ok(image)
}


fn decode_image(encoded_image: &EncodedImage) -> Result<RawImage> {

    let EncodedImage { 
        target_format, encoding, palettes, bytes
    } = encoded_image;

    let mut format = target_format.clone();
    let mut decoded_bytes = match encoding {
        Encoding::RAW => bytes.clone(),
        Encoding::RLE => decode_RLE(&encoded_image)?,
        Encoding::JPEG => decode_JPEG(&encoded_image)?,
        Encoding::JPEG2000 => decode_JPEG2000(&encoded_image)?
    };

    if let Some(ref palettes) = palettes {
        decoded_bytes = map_to_palette(&decoded_bytes, palettes, format.channel_depth)?;
        format.channels = 3;
        format.channel_depth = 2;
    }

    Ok(RawImage { format, bytes: decoded_bytes })
}

#[allow(non_snake_case)]
fn decode_JPEG(encoded_image: &EncodedImage) -> Result<Vec<u8>> {

    let mut decoder = jpeg_decoder::Decoder::new(encoded_image.bytes.as_slice());
    let decoded_pixel_data = decoder.decode()?;
    Ok(decoded_pixel_data)
}

#[allow(non_snake_case)]
fn decode_JPEG2000(encoded_image: &EncodedImage) -> Result<Vec<u8>> {

    let Format { channels, channel_depth, .. } = encoded_image.target_format;

    let dynamic_image = jpeg2000::decode::from_memory(
        encoded_image.bytes.as_slice(),
        Codec::JP2,
        DecodeConfig {
            default_colorspace: None,
            discard_level: 0,
        },
        None,
    )?;

    let pixel_data: Vec<u8> = match (channels, channel_depth) {
        (1, 1) => dynamic_image.to_luma().into_vec(),
        (3, 1) => dynamic_image.to_rgb().into_vec(),
        (4, 1) => dynamic_image.to_rgba().into_vec(),
        _ => return Err(anyhow!(
            "JPEG2000 output is unsupported: {} channels of depth {} bytes",
            channels, channel_depth
        ))
    };

    Ok(pixel_data)
}

fn map_to_palette(bytes: &Vec<u8>, palettes: &Palettes, channel_depth: u32) -> Result<Vec<u8>> {

    match channel_depth {
        1 | 2 => (),
        _ => return Err(anyhow!(
            "Unsupported bit depth: {} (1 and 2 supported)",
            channel_depth
        )) 
    };

    let d: usize = channel_depth.try_into().unwrap();
    let mapped_bytes: Vec<u8> = bytes
        .chunks_exact(d)
        .map(|v| -> usize { match v {
            [b0] => u8::from_be_bytes([*b0]).into(),
            [b0, b1] => u16::from_be_bytes([*b0, *b1]).into(),
            _ => panic!()
        }})
        .flat_map(|index| {

            let [r0, r1] = palettes[0][index].to_le_bytes();
            let [g0, g1] = palettes[1][index].to_le_bytes();
            let [b0, b1] = palettes[2][index].to_le_bytes();

            vec![r0, r1, g0, g1, b0, b1]
        })
        .collect();

    Ok(mapped_bytes)
}

#[allow(non_snake_case)]
fn decode_RLE(encoded_data: &EncodedImage) -> Result<Vec<u8>> {

    let EncodedImage { bytes, .. } = encoded_data;

    /*
        Decoding segments
    */

    let mut offsets = decode_header(&bytes)?;
    offsets.push(bytes.len());
    let nb_segments = offsets.len();

    let mut decoded_segments = Vec::new();
    for i in 0..nb_segments-1 {
        let o1 = offsets[i];
        let o2 = offsets[i+1];
        let segment_data = &bytes[o1..o2];
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
        let x = (i * 4).try_into().unwrap();
        let o = u32::from_le_bytes((&pixel_data[x..x+4]).try_into()?);
        offsets.push(o.try_into().unwrap());
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

            let n = (3 + (255 - header_val)).into();
            let rep_byte = segment_data[i];
            decoded_segment.append(&mut vec![rep_byte; n]);
            i += 1;

        } else if header_val < 129 {

            let n: usize = header_val.into();
            if i+n > N { break; }
            decoded_segment.append(&mut segment_data[i..i+n].to_vec());
            i += n;

        }
    }

    Ok(decoded_segment)
}
