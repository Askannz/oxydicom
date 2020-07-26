use std::path::PathBuf;
use std::fmt::{Debug, Display};
use dicom::object::open_file;
use dicom::object::mem::InMemDicomObject;
use dicom::core::value::{Value, PrimitiveValue, C};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::core::dictionary::DataDictionary;
use iced::{
    Container, Element, Settings, Image, Row,
    Text, Scrollable, scrollable, Button, Column, button,
    Length, HorizontalAlignment, VerticalAlignment, Align,
    Application, executor, Command, window, Color, Background
};
use iced::image::Handle;
use clipboard::{ClipboardProvider, ClipboardContext};

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
    image_handle: Handle,
    filepath: String,
    table: Vec<(String, String)>,
    show_tags: bool,
    clipoard: ClipboardContext,
    states: States
}

struct States {
    scroll: scrollable::State,
    show_tags_button: button::State,
    table_buttons: Vec<(button::State, button::State)>
}

struct Flags {
    filepath: String,
    image: RawImage,
    table: Vec<(String, String)>
}

#[derive(Debug, Clone)]
enum Message {
    TagsTogglePressed,
    TableCellPressed(String)
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
        let image_handle = Handle::from_pixels(w, h, bytes);

        let table_buttons_states = table
            .iter().map(|_| (button::State::new(), button::State::new()))
            .collect();

        let states = States {
            scroll: scrollable::State::new(),
            show_tags_button: button::State::new(),
            table_buttons: table_buttons_states
        };

        let app = App { 
            image_handle,
            filepath,
            table,
            show_tags: false,
            clipoard: clipboard::ClipboardProvider::new().unwrap(),
            states
        };

        (app, Command::none())
    }

    fn title(&self) -> String {
        String::from("DICOM")
    }

    fn update(&mut self, message: Message) -> Command<Self::Message> {
        match message {
            Message::TagsTogglePressed => self.show_tags = !self.show_tags,
            Message::TableCellPressed(txt) => self.clipoard.set_contents(txt).unwrap()
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message> {

        let image = Image::new(self.image_handle.clone());

        let header = Row::new()
            .push(
                Button::new(&mut self.states.show_tags_button, Text::new("Tags"))
                    .on_press(Message::TagsTogglePressed)
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

            let mut tags_col_element = Vec::<Element<Message>>::new();
            let mut vals_col_element = Vec::<Element<Message>>::new();

            let iterator = self.table.iter().zip(self.states.table_buttons.iter_mut());

            for (i, ((tag_str, val_str), (tag_state, val_state))) in iterator.enumerate() {

                let stylesheet = match i % 2 {
                    0 => ButtonStyleSheet::Light,
                    _ => ButtonStyleSheet::Dark,
                };

                tags_col_element.push(
                    Button::new(
                        tag_state,
                        Text::new(tag_str)
                    )
                    .width(Length::Fill)
                    .style(stylesheet.clone())
                    .on_press(Message::TableCellPressed(tag_str.into()))
                    .into()  
                );

                vals_col_element.push(
                    Button::new(
                        val_state,
                        Text::new(val_str)
                    )
                    .width(Length::Fill)
                    .style(stylesheet)
                    .on_press(Message::TableCellPressed(val_str.into()))
                    .into()  
                )

            }

            let row = Row::new()
                .push(
                    Column::with_children(tags_col_element)
                        .width(Length::FillPortion(2))
                )
                .push(
                    Column::with_children(vals_col_element)
                        .width(Length::FillPortion(2))
                );

            Scrollable::new(&mut self.states.scroll)
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

#[derive(Clone)]
enum ButtonStyleSheet {
    Light,
    Dark
}


impl button::StyleSheet for ButtonStyleSheet {
    fn active(&self) -> button::Style {

        let val = match self {
            Self::Light => 0.8,
            Self::Dark => 0.7
        };

        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(val, val, val),
            )),
            //text_color: Color::BLACK,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(1.0, 1.0, 1.0),
            )),
            //text_color: Color::BLACK,
            ..button::Style::default()
        }
    }
}

fn get_dicom_table(dicom: &Dicom) -> Vec<(String, String)> {
    let root = dicom.clone().into_inner();
    get_formatted_list(0, &root)
}

fn get_formatted_list(depth: usize, root: &MemDicom) -> Vec<(String, String)> {

    let dict = StandardDataDictionary;
    let mut table = Vec::<(String, String)>::new();

    let pad_depth = |s: String| format!("{}{}", " ".repeat(4*depth), s);

    for element in root {

        let tag = element.header().tag;

        let tag_name_str = dict
            .by_tag(tag.clone())
            .map(|entry| entry.alias)
            .unwrap_or("Unknown");

        let tag_str = pad_depth(format!("{} {}", tag, tag_name_str));
        let val_str = format_value(element.value());

        table.push((tag_str, val_str));

        if let Value::Sequence { items, .. } = element.value() {
            for item in items {
                table.push((" -".to_owned(), " -".to_owned()));
                let mut sub_table = get_formatted_list(depth + 1, item);
                table.append(&mut sub_table);
            }
            table.push((" -".to_owned(), " -".to_owned()));
        }
    }

    table
}

type MemDicom = InMemDicomObject<StandardDataDictionary>;

fn format_value<P>(value: &Value<MemDicom, P>) -> String {

    match value {

        Value::Primitive(prim_val) => format_primitive(prim_val),
        Value::Sequence { .. } => "<sequence>".to_owned(),
        Value::PixelSequence { .. } => "<pixel sequence>".to_owned()
    }
}

fn format_primitive(prim_val: &PrimitiveValue) -> String {

    const MAX_LEN: usize = 40;

    let mut s = match prim_val {

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
    };

    if s.len() > MAX_LEN {
        s = format!("{} <...>", &s[..MAX_LEN]);
    }

    s
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