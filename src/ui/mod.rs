use gcfeeder::feeder;

use iced::{
    button, scrollable, Align, Button, Column, Container, Element, Length, Sandbox, Scrollable,
    Text,
};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use feeder::Feeder;
use thread::JoinHandle;

mod style;

pub struct FeederThread {
    handle: Option<JoinHandle<()>>,
    sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<feeder::Error>,
}

impl FeederThread {
    pub fn spawn() -> Result<FeederThread, feeder::Error> {
        let (terminate_send, terminate_recv) = mpsc::channel();
        let (error_send, error_recv) = mpsc::channel();

        let mut feeder = Feeder::new()?;

        let handle = thread::spawn(move || loop {
            if terminate_recv.try_recv().is_ok() {
                break;
            }

            if let Err(err) = feeder.feed() {
                error_send
                    .send(err)
                    .expect("failed to send error from feeder thread");
                break;
            }

            thread::sleep(Duration::from_millis(2));
        });

        Ok(FeederThread {
            handle: Some(handle),
            sender: terminate_send,
            receiver: error_recv,
        })
    }

    pub fn error(&self) -> Option<feeder::Error> {
        self.receiver.try_recv().ok()
    }
}

impl Drop for FeederThread {
    fn drop(&mut self) {
        self.sender
            .send(())
            .expect("failed to terminate feeder thread");

        if let Some(handle) = self.handle.take() {
            handle.join().expect("failed to join feeder thread");
        }
    }
}

#[derive(Default)]
pub struct GCFeeder {
    feeder_thread: Option<FeederThread>,
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
                if self.feeder_thread.is_some() {
                    return;
                }

                match FeederThread::spawn() {
                    Ok(feeder_thread) => {
                        self.feeder_thread = Some(feeder_thread);
                    }
                    Err(err) => {
                        self.log(&format!("{:?}", err));
                    }
                }
            }
            Message::StopThread => {
                self.feeder_thread = None;
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

        let feeder_status = Text::new(if self.feeder_thread.is_some() {
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
                .push(error_log)
                .push(clear_button)
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
