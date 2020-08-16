use std::path::PathBuf;
use std::fmt::Debug;
use dicom::object::open_file;
use iced::{
    Container, Element, Settings, Image, Row,
    Text, Scrollable, scrollable, Button, Column, button,
    Length, HorizontalAlignment, VerticalAlignment, Align,
    Application, executor, Command, window, Color,
};
use iced::image::Handle;
use clipboard::{ClipboardProvider, ClipboardContext};
use anyhow::{Result, anyhow};

mod utils;
mod decoding;
mod dicom_table;
mod ui;

use utils::{Format, RawImage, convert_to_BGRA8888};
use decoding::get_image;
use dicom_table::get_dicom_table;

pub fn main() -> Result<()> {

    let mut args = std::env::args().skip(1);

    let input_path = PathBuf::from(
        args.next().ok_or(anyhow!("You must specify a file to open."))?);

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

    Ok(App::run(settings))
}

struct App {
    image_handle: Handle,
    filepath: String,
    table: Vec<[String; 3]>,
    show_tags: bool,
    clipoard: ClipboardContext,
    states: States
}

struct States {
    scroll: scrollable::State,
    show_tags_button: button::State,
    table_buttons: Vec<[button::State; 3]>
}

struct Flags {
    filepath: String,
    image: RawImage,
    table: Vec<[String; 3]>
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

        let image_data_bgra = convert_to_BGRA8888(&image).unwrap();

        let RawImage { format, bytes } = image_data_bgra;
        let Format { h, w, .. } = format;
        let image_handle = Handle::from_pixels(w, h, bytes);

        let table_buttons_states = table
            .iter().map(|_| [
                button::State::new(),
                button::State::new(),
                button::State::new()
            ])
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
                Button::new(&mut self.states.show_tags_button, Text::new("Tags").color(Color::WHITE))
                    .on_press(Message::TagsTogglePressed)
                    .style(ui::TagsButtonStyleSheet)
            )
            .push(
                Container::new(
                    Text::new(self.filepath.clone())
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .color(Color::WHITE)
                )
                .width(Length::Fill)
                .padding(5)
                .center_x()
            )
            .padding(20);

        let content: Element<Message> = if self.show_tags {

            const FILL_W: [u16; 3] = [1, 3, 3];

            let mut rows = Vec::<Element<Message>>::new();
            let iterator = self.table.iter().zip(self.states.table_buttons.iter_mut());

            for (i, (strings, states)) in iterator.enumerate() {

                let stylesheet = match i % 2 {
                    0 => ui::CellButtonStyleSheet::Light,
                    _ => ui::CellButtonStyleSheet::Dark,
                };

                let iterator = strings.into_iter().zip(states.into_iter());

                let row = Row::with_children(iterator.enumerate().map(|(x, (s, state))| {

                    Button::new(
                        state,
                        Text::new(s)
                            .height(Length::Fill)
                            .color(Color::WHITE)
                            .size(16)
                    )
                    .style(stylesheet.clone())
                    .on_press(Message::TableCellPressed(s.clone()))
                    .width(Length::FillPortion(FILL_W[x]))
                    .into()

                }).collect())
                .width(Length::Fill)
                .into();

                rows.push(row);

            }

            let col = Column::with_children(rows)
                .spacing(2)
                .width(Length::Fill);

            Scrollable::new(&mut self.states.scroll)
                .push(col)
                .width(Length::Fill)
                .into()

        } else {
            Container::new(image)
                .style(ui::ContainerStyleSheet)
                .into()
        };

        Container::new(
            Column::new()
                .push(header)
                .push(content)
                .align_items(Align::Start)
                .width(Length::Fill)
        )
        .style(ui::ContainerStyleSheet)
        .height(Length::Fill)
        .into()
    }
} 
