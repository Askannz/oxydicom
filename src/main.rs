use std::path::PathBuf;
use dicom::object::open_file;
use iced::{Container, Element, Sandbox, Settings, Image};
use iced::image::Handle;

mod utils;
mod decoding;

use utils::{DecodedImageData, convert_to_BGRA};
use decoding::get_image;

pub fn main() {
    Tiger::run(Settings::default())
}

struct Tiger {
    image_data_bgra: DecodedImageData
}

impl Sandbox for Tiger {
    type Message = ();

    fn new() -> Self {

        let mut args = std::env::args().skip(1);

        let input_path = PathBuf::from(
            args.next().expect("Not enough arguments"));
    
        let dicom = open_file(input_path.as_os_str()).unwrap();
        let image_data = get_image(dicom).unwrap();
        let image_data_bgra = convert_to_BGRA(&image_data).unwrap();

        Tiger {
            image_data_bgra
        }
    }

    fn title(&self) -> String {
        String::from("DICOM")
    }

    fn update(&mut self, _message: ()) {}

    fn view(&mut self) -> Element<()> {

        let DecodedImageData { w, h, pixel_data, .. } = &self.image_data_bgra;
        let handle = Handle::from_pixels(*w, *h, pixel_data.clone());

        let image = Image::new(handle);

        Container::new(image)
            .into()
    }
} 
