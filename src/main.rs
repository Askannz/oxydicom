use std::path::PathBuf;
use dicom::object::open_file;
use dicom::core::value::{Value, PrimitiveValue};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::core::dictionary::DataDictionary;
use iced::{
    Container, Element, Sandbox, Settings, Image, Row,
    Text, Scrollable, scrollable, Button, Column, button,
    Length, HorizontalAlignment, VerticalAlignment, Align
};
use iced::image::Handle;

mod utils;
mod decoding;

use utils::{DecodedImageData, convert_to_BGRA, Dicom};
use decoding::get_image;

pub fn main() {
    App::run(Settings::default())
}

struct App {
    handle: Handle,
    filepath: String,
    table_string: String,
    scroll_state: scrollable::State,
    button_state: button::State,
    show_tags: bool
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ButtonPressed
}

impl Sandbox for App {
    type Message = Message;

    fn new() -> Self {

        let mut args = std::env::args().skip(1);

        let input_path = PathBuf::from(
            args.next().expect("Not enough arguments"));
    
        let dicom = open_file(input_path.as_os_str()).unwrap();
        let table_string = get_dicom_table_string(&dicom);
        let image_data = get_image(dicom).unwrap();
        let image_data_bgra = convert_to_BGRA(&image_data).unwrap();

        let DecodedImageData { w, h, pixel_data, .. } = image_data_bgra;
        let handle = Handle::from_pixels(w, h, pixel_data);

        App { 
            handle,
            filepath: input_path.as_os_str().to_str().unwrap().to_owned(),
            table_string,
            scroll_state: scrollable::State::new(),
            button_state: button::State::new(),
            show_tags: false
        }
    }

    fn title(&self) -> String {
        String::from("DICOM")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::ButtonPressed => self.show_tags = !self.show_tags
        }
    }

    fn view(&mut self) -> Element<Message> {

        let image = Image::new(self.handle.clone());

        let header = Row::new()
            .push(
                Button::new(&mut self.button_state, Text::new("Tags"))
                    .on_press(Message::ButtonPressed)
            )
            .push(
                Container::new(
                    Text::new(self.filepath.clone())
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                )
                .padding(5)
            )
            .align_items(Align::Center)
            .padding(5);

        let content: Element<Message> = if self.show_tags {
            Scrollable::new(&mut self.scroll_state)
                .push(Text::new(self.table_string.as_str()))
                .into()
        } else {
            Container::new(image)
            //Text::new("AAAA".to_owned())
                .into()
        };

        Column::new()
            .push(header)
            .push(content)
            //.width(Length::from(600))
            .into()
    }
} 


fn get_dicom_table_string(dicom: &Dicom) -> String {

    let dict = StandardDataDictionary;

    let table: Vec<(String, String)> = dicom.clone().into_iter().map(|element| {

        let tag = element.header().tag;

        let tag_name_str = dict
            .by_tag(tag.clone())
            .map(|entry| entry.alias)
            .unwrap_or("Unknown");

        let tag_str = format!("{} {}", tag, tag_name_str);

        let val_str = match element.value() {

            Value::Primitive(val) => {
                /*if let Some(v) = val.int32() {
                    format!("{}", v)
                } else if let Some(s) = val.string() {
                    s.to_owned()
                } else {
                    "<unknown>".to_owned()
                }*/
                format!("{:?}", val)
            },

            Value::Sequence { .. } => "Sequence".to_owned(),
            Value::PixelSequence { .. } => "Pixel Sequence".to_owned()
        };

        (tag_str, val_str)

    }).collect();

    let max_tag_len = table.iter().map(|(tag_str, _)| tag_str.len()).max().unwrap();

    let string_vec: Vec<String> = table
        .iter()
        .map(|(tag_str, val_str)| format!("{:width$} : {}", tag_str, val_str, width = max_tag_len))
        .collect();
    
    string_vec.join("\n")
}