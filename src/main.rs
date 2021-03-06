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
use dicom_table::{TableEntry, get_dicom_table};

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
    table: Vec<TableEntry>,
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
    table: Vec<TableEntry>
}

#[derive(Debug, Clone)]
enum Message {
    TagsTogglePressed,
    TableCellPressed(Option<String>)
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
            Message::TableCellPressed(Some(txt)) => self.clipoard.set_contents(txt).unwrap(),
            Message::TableCellPressed(None) => ()
        }

        Command::none()
    }

    fn view(&mut self) -> Element<Message> {

        let image = Image::new(self.image_handle.clone());

        let States {
            show_tags_button,
            table_buttons,
            scroll
        } = &mut self.states;

        let header = make_header(&self.filepath, show_tags_button);

        let content: Element<Message> = if self.show_tags {

            make_tags_content(&self.table, table_buttons, scroll)

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


fn make_tags_content<'a>(
    table: &Vec<TableEntry>,
    table_buttons: &'a mut Vec<[button::State; 3]>,
    scroll: &'a mut scrollable::State
) -> Element<'a, Message> {

    const FILL_W: [u16; 3] = [1, 3, 3];

    let mut rows = Vec::<Element<Message>>::new();
    let row_iterator = table.iter()
        .zip(table_buttons.iter_mut())
        .enumerate();

    for (i, (table_entry, states)) in row_iterator {

        let stylesheet = match i % 2 {
            0 => ui::CellButtonStyleSheet::Light,
            _ => ui::CellButtonStyleSheet::Dark,
        };

        let display_values = vec![
            &table_entry.tag_key,
            &table_entry.tag_name,
            &table_entry.short_value
        ];

        let clipboard_values = vec![
            Some(table_entry.tag_key.clone()),
            Some(table_entry.tag_name.clone()),
            table_entry.value.clone()
        ];

        let col_iterator = display_values.into_iter()
            .zip(clipboard_values.into_iter())
            .zip(states.into_iter())
            .enumerate();

        let row = Row::with_children(col_iterator.map(|(j, ((disp_val, clip_val), state))| {

            Button::new(
                state,
                Text::new(disp_val)
                    .height(Length::Fill)
                    .color(Color::WHITE)
                    .size(16)
            )
            .style(stylesheet.clone())
            .on_press(Message::TableCellPressed(clip_val))
            .width(Length::FillPortion(FILL_W[j]))
            .into()

        }).collect())
        .width(Length::Fill)
        .into();

        rows.push(row);
    }

    let tags_col = Column::with_children(rows)
        .spacing(2)
        .width(Length::Fill);

    let col = Column::with_children(vec![
        Text::new("Click a cell to copy to clipboard")
            .color(Color::WHITE)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
        tags_col.into()
    ])
    .align_items(Align::Center);

    Scrollable::new(scroll)
        .push(col)
        .width(Length::Fill)
        .into()
}


fn make_header<'a>(filepath: &String, button_state: &'a mut button::State) -> Row<'a, Message> {

    Row::new()
    .push(
        Button::new(button_state, Text::new("Tags").color(Color::WHITE))
            .on_press(Message::TagsTogglePressed)
            .style(ui::TagsButtonStyleSheet)
    )
    .push(
        Container::new(
            Text::new(filepath.clone())
                .horizontal_alignment(HorizontalAlignment::Center)
                .vertical_alignment(VerticalAlignment::Center)
                .color(Color::WHITE)
        )
        .width(Length::Fill)
        .padding(5)
        .center_x()
    )
    .padding(20)
}
