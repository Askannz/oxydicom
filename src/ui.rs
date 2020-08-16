use iced::{
    button, Color, Background, container
};


#[derive(Clone)]
pub enum CellButtonStyleSheet {
    Light,
    Dark
}


impl button::StyleSheet for CellButtonStyleSheet {
    fn active(&self) -> button::Style {

        let val = match self {
            Self::Light => 0.2,
            Self::Dark => 0.1
        };

        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(val, val, val),
            )),
            border_width: 0,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(0.6, 0.6, 0.6),
            )),
            border_width: 0,
            ..button::Style::default()
        }
    }
}

pub struct TagsButtonStyleSheet;
impl button::StyleSheet for TagsButtonStyleSheet {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(0.2, 0.2, 0.2),
            )),
            border_width: 0,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(
                Color::from_rgb(0.6, 0.6, 0.6),
            )),
            border_width: 0,
            ..button::Style::default()
        }
    }
}

pub struct ContainerStyleSheet;
impl container::StyleSheet for ContainerStyleSheet {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::BLACK)),
            ..container::Style::default()
        }
    }
}
