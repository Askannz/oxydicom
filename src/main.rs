use std::path::PathBuf;
use std::fmt::{Debug, Display};
use dicom::object::open_file;
use dicom::core::value::{Value, PrimitiveValue, C};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::core::dictionary::DataDictionary;
use iced::{
    Container, Element, Settings, Image, Row,
    Text, Scrollable, scrollable, Button, Column, button,
    Length, HorizontalAlignment, VerticalAlignment, Align,
    Application, executor, Command, window
};
use iced::image::Handle;

mod utils;
mod decoding;

use utils::{Format, RawImage, convert_to_BGRA, Dicom};
use decoding::get_image;

pub fn main() {

    let mut args = std::env::args().skip(1);

    let input_path = PathBuf::from(
        args.next().expect("Not enough arguments"));

    let filepath = input_path.as_os_str().to_str().unwrap().to_owned();

    let dicom = open_file(input_path.as_os_str()).unwrap();
    let table = get_dicom_table(&dicom);
    let image = get_image(&dicom).unwrap();

    let Format { w, h, .. } = image.format;

    let flags = Flags { 
        image,
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
    image: RawImage,
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

        let Flags { filepath, image, table } = flags;

        let image_data_bgra = convert_to_BGRA(&image).unwrap();

        let RawImage { format, bytes } = image_data_bgra;
        let Format { h, w, .. } = format;
        let handle = Handle::from_pixels(w, h, bytes);

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
        let val_str = format_value(element.value());

        (tag_str, val_str)

    }).collect();

    table
}

fn format_value<I, P>(value: &Value<I, P>) -> String {

    match value {

        Value::Primitive(val) => match val {

            PrimitiveValue::Empty => "<empty>".to_owned(),
            PrimitiveValue::Strs(arr) => format_array(arr),
            PrimitiveValue::Str(s) => s.clone(),
            PrimitiveValue::Tags(arr) => format_array(arr),
            PrimitiveValue::U8(arr) => format_array(arr),
            PrimitiveValue::I16(arr) => format_array(arr),
            PrimitiveValue::U16(arr) => format_array(arr),
            PrimitiveValue::I32(arr) => format_array(arr),
            PrimitiveValue::U32(arr) => format_array(arr),
            PrimitiveValue::I64(arr) => format_array(arr),
            PrimitiveValue::U64(arr) => format_array(arr),
            PrimitiveValue::F32(arr) => format_array(arr),
            PrimitiveValue::F64(arr) => format_array(arr),
            PrimitiveValue::Date(arr) => format_array(arr),
            PrimitiveValue::DateTime(arr) => format_array(arr),
            PrimitiveValue::Time(arr) => format_array(arr)
        },

        Value::Sequence { .. } => "<sequence>".to_owned(), // TODO: properly implement this
        Value::PixelSequence { .. } => "<pixel sequence>".to_owned()
    }
}

fn format_array<T: Display>(arr: &C<T>) -> String {

    match arr.len() {
        0 => "[]".to_owned(),
        1 => format!("{}", arr[0]),
        _ => {
            let repr_list: Vec<String> = arr
                .iter()
                .map(|v| format!("{}", v))
                .collect();
            repr_list.join(",")
        }
    }.trim_end_matches(char::from(0)).to_owned()
}