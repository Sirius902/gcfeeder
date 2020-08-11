use gcfeeder::feeder;

use feeder::Feeder;
use iced::{
    button, executor, scrollable, time, Align, Application, Button, Column, Command, Container,
    Element, Length, Row, Scrollable, Subscription, Text,
};
use std::time::Duration;

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

    fn log_error(&mut self, error: feeder::Error) {
        self.log(&format!("{:?}", error));
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    StartThread,
    StopThread,
    ClearLog,
    CheckError,
}

impl Application for GCFeeder {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (Self::default(), Command::none())
    }

    fn title(&self) -> String {
        "gcfeeder".to_owned()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::StartThread => {
                if self.feeder.is_none() {
                    match Feeder::new() {
                        Ok(feeder) => {
                            self.feeder = Some(feeder);
                        }
                        Err(err) => {
                            self.log_error(err);
                        }
                    }
                }
            }
            Message::StopThread => {
                self.feeder = None;
            }
            Message::ClearLog => {
                self.log_text.clear();
            }
            Message::CheckError => {
                let mut stop_feeder = false;

                if let Some(ref feeder) = self.feeder {
                    if let Ok(err) = feeder.error_receiver.try_recv() {
                        self.log_error(err);
                        stop_feeder = true;
                    }
                }

                if stop_feeder {
                    self.feeder = None;
                }
            }
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(150)).map(|_| Message::CheckError)
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
