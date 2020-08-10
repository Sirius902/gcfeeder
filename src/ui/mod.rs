use gcfeeder::feeder;

use feeder::Feeder;
use iced::{
    button, scrollable, Align, Button, Column, Container, Element, Length, Row, Sandbox,
    Scrollable, Text,
};

mod style;

#[derive(Default)]
pub struct GCFeeder {
    feeder: Option<Feeder>,
    log_text: String,
    error_log: scrollable::State,
    clear_button: button::State,
    start_button: button::State,
    stop_button: button::State,
}

impl GCFeeder {
    fn log(&mut self, message: &str) {
        self.log_text.push_str(message);
        self.log_text.push('\n');
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    StartThread,
    StopThread,
    ClearLog,
}

impl Sandbox for GCFeeder {
    type Message = Message;

    fn new() -> Self {
        Self::default()
    }

    fn title(&self) -> String {
        "gcfeeder".to_owned()
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::StartThread => {
                if self.feeder.is_some() {
                    return;
                }

                match Feeder::new() {
                    Ok(feeder) => {
                        self.feeder = Some(feeder);
                    }
                    Err(err) => {
                        self.log(&format!("{:?}", err));
                    }
                }
            }
            Message::StopThread => {
                self.feeder = None;
            }
            Message::ClearLog => {
                self.log_text.clear();
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        let error_log = Container::new(
            Scrollable::new(&mut self.error_log)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(style::dark::Scrollable)
                .push(Text::new(&self.log_text)),
        )
        .width(Length::from(300))
        .height(Length::from(225))
        .padding(10)
        .style(style::dark::ErrorLog);

        let clear_button = Button::new(&mut self.clear_button, Text::new("Clear"))
            .style(style::dark::Button)
            .on_press(Message::ClearLog);

        let feeder_status = Text::new(if self.feeder.is_some() {
            "Running"
        } else {
            "Idle"
        })
        .size(24);

        let start_button = Button::new(&mut self.start_button, Text::new("Start"))
            .style(style::dark::Button)
            .on_press(Message::StartThread);

        let stop_button = Button::new(&mut self.stop_button, Text::new("Stop"))
            .style(style::dark::Button)
            .on_press(Message::StopThread);

        Container::new(
            Column::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .padding(15)
                .spacing(5)
                .align_items(Align::Center)
                .push(
                    Row::new()
                        .spacing(15)
                        .push(Text::new("Error Log"))
                        .push(clear_button),
                )
                .push(error_log)
                .push(feeder_status)
                .push(start_button)
                .push(stop_button),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::dark::Container)
        .center_x()
        .into()
    }
}
