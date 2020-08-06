use gcfeeder::feeder;

use iced::{button, Align, Button, Column, Container, Element, Length, Sandbox, Text};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use feeder::Feeder;
use thread::JoinHandle;

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
    feeder_errors: Vec<feeder::Error>,
    start_button: button::State,
    stop_button: button::State,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    StartThread,
    StopThread,
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
                        self.feeder_errors.push(err);
                    }
                }
            }
            Message::StopThread => {
                self.feeder_thread = None;
            }
        }
    }

    fn view(&mut self) -> Element<Message> {
        Container::new(
            Column::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .padding(15)
                .spacing(5)
                .align_items(Align::Center)
                .push(
                    Text::new(if self.feeder_thread.is_some() {
                        "Running"
                    } else {
                        "Idle"
                    })
                    .size(24),
                )
                .push(
                    Button::new(&mut self.start_button, Text::new("Start"))
                        .on_press(Message::StartThread),
                )
                .push(
                    Button::new(&mut self.stop_button, Text::new("Stop"))
                        .on_press(Message::StopThread),
                ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .into()
    }
}
