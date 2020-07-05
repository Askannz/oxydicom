use std::path::PathBuf;
use dicom::object::open_file;
use dicom::core::value::{Value, PrimitiveValue};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::core::dictionary::DataDictionary;
use iced::{
    Container, Element, Sandbox, Settings, Image, Row,
    Text, Scrollable, scrollable, Button, Column, button,
    Length, HorizontalAlignment, VerticalAlignment, Align,
    Application, executor, Command, window, Space
};
use iced::image::Handle;

mod utils;
mod decoding;

use utils::{DecodedImageData, convert_to_BGRA, Dicom};
use decoding::get_image;

pub fn main() {

    let mut args = std::env::args().skip(1);

    let input_path = PathBuf::from(
        args.next().expect("Not enough arguments"));

    let filepath = input_path.as_os_str().to_str().unwrap().to_owned();

    let dicom = open_file(input_path.as_os_str()).unwrap();
    let table = get_dicom_table(&dicom);
    let image_data = get_image(dicom).unwrap();

    let DecodedImageData { w, h, .. } = image_data;

    let flags = Flags { 
        image_data,
        table,
        filepath
    };

    let settings = Settings {
        flags: flags,
        window: window::Settings {
            size: (w, h + 60),
            resizable: true,
            decorations: true,
            ..Default::default()
        },
        default_font: None,
        antialiasing: true
    };

    App::run(settings)
}

struct App {
    handle: Handle,
    filepath: String,
    table: Vec<(String, String)>,
    scroll_state: scrollable::State,
    button_state: button::State,
    show_tags: bool
}

struct Flags {
    filepath: String,
    image_data: DecodedImageData,
    table: Vec<(String, String)>
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ButtonPressed
}

impl Application for App {
    type Executor = executor::Null;
    type Message = Message;
    type Flags = Flags;

    fn new(flags: Flags) -> (Self, Command<Self::Message>) {

        let Flags { filepath, image_data, table } = flags;

        let image_data_bgra = convert_to_BGRA(&image_data).unwrap();

        let DecodedImageData { w, h, pixel_data, .. } = image_data_bgra;
        let handle = Handle::from_pixels(w, h, pixel_data);

        let app = App { 
            handle,
            filepath,
            table,
            scroll_state: scrollable::State::new(),
            button_state: button::State::new(),
            show_tags: false
        };

        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("DICOM")
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
            Message::ButtonPressed => self.show_tags = !self.show_tags
        }

        Command::none()
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
                //.padding(5)
            )
            .align_items(Align::Center);
            //.padding(5);

        let content: Element<Message> = if self.show_tags {

            let tags_col_element: Vec<Element<Message>> = self.table.iter().map(|(tag_str, _)| {
                Text::new(tag_str)
                    .into()
            }).collect();

            let vals_col_element: Vec<Element<Message>> = self.table.iter().map(|(_, val_str)| {
                Text::new(val_str)
                    .into()
            }).collect();

            let row = Row::new()
                .push(
                    Column::with_children(tags_col_element)
                        .width(Length::FillPortion(2))
                )
                .push(
                    Column::with_children(vals_col_element)
                        .width(Length::FillPortion(2))
                );

            Scrollable::new(&mut self.scroll_state)
                .push(row)
                .width(Length::Fill)
                .height(Length::Shrink)
                .into()

        } else {
            Container::new(image)
                .into()
        };

        Column::new()
            .push(header)
            .push(content)
            .align_items(Align::Center)
            .width(Length::Fill)
            .into()
    }
} 


fn get_dicom_table(dicom: &Dicom) -> Vec<(String, String)> {

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

    table
}